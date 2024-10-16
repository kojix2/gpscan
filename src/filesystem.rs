// External crates
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use sysinfo::Disks;

// Standard library imports
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::io;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::time::SystemTime;

use crate::xml::xml_escape;

// Constants representing GrandPerspective version information
const GRANDPERSPECTIVE_APP_VERSION: &str = "4";
const GRANDPERSPECTIVE_FORMAT_VERSION: &str = "7";

// Retrieves the device ID from metadata (Linux).
#[cfg(target_os = "linux")]
fn get_device_id(metadata: &Metadata) -> u64 {
    metadata.st_dev()
}

// Retrieves the device ID from metadata (Unix).
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
fn get_device_id(metadata: &Metadata) -> u64 {
    metadata.dev()
}

// Retrieves the device ID from metadata (Windows).
#[cfg(windows)]
fn get_device_id(metadata: &Metadata) -> u64 {
    metadata.volume_serial_number().unwrap_or(0) as u64
}

// Retrieves the inode number from metadata (Linux).
#[cfg(target_os = "linux")]
fn get_inode_number(metadata: &Metadata) -> u64 {
    metadata.st_ino()
}

// Retrieves the inode number from metadata (Unix).
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
fn get_inode_number(metadata: &Metadata) -> u64 {
    metadata.ino()
}

// Retrieves the inode number from metadata (Windows).
#[cfg(target_os = "windows")]
fn get_inode_number(metadata: &Metadata) -> u64 {
    // Note: Windows does not have inodes. Use the file index instead.
    metadata.file_index().unwrap_or(0)
}

/// Runs the main logic of the program.
pub fn run(matches: ArgMatches) -> io::Result<()> {
    // Get the directory path from arguments
    let directory = matches
        .get_one::<String>("directory")
        .expect("Directory path is required")
        .as_str();

    let root_path = Path::new(directory);

    // Check if the provided path exists
    if !root_path.exists() {
        eprintln!(
            "[gpscan] Error: The specified path does not exist: {}",
            root_path.display()
        );
        std::process::exit(1); // Exit code 1 for non-existent path
    }

    // Check if the provided path is a directory
    if !root_path.is_dir() {
        eprintln!(
            "[gpscan] Error: The specified path is not a directory: {}",
            root_path.display()
        );
        std::process::exit(1); // Exit code 1 for invalid directory
    }

    // Get option values
    let apparent_size_flag = matches.get_flag("apparent-size");
    let cross_mount_points = matches.get_flag("mounts");
    let include_zero_files = matches.get_flag("include-zero-files");
    let include_empty_folders = matches.get_flag("include-empty-folders");

    // Get the device ID of the root directory
    let root_metadata = fs::metadata(root_path)?;
    let root_dev = get_device_id(&root_metadata);

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(root_path, &disks);

    // Output XML header
    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        r#"<GrandPerspectiveScanDump appVersion="{}" formatVersion="{}">"#,
        GRANDPERSPECTIVE_APP_VERSION, GRANDPERSPECTIVE_FORMAT_VERSION
    );
    let scan_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    println!(
        r#"<ScanInfo volumePath="{}" volumeSize="{}" freeSpace="{}" scanTime="{}" fileSizeMeasure="physical">"#,
        xml_escape(&volume_path),
        volume_size,
        free_space,
        scan_time
    );

    // Create a set to store visited inodes
    let mut visited_inodes = HashSet::new();

    // Start traversing the directory with new options
    traverse_directory_to_xml(
        root_path,
        true,
        root_dev,
        apparent_size_flag,
        cross_mount_points,
        include_zero_files,
        include_empty_folders,
        &mut visited_inodes,
    )?;

    // Close XML tags
    println!("</ScanInfo>");
    println!("</GrandPerspectiveScanDump>");

    Ok(())
}

/// Retrieves volume information for the given path.
fn get_volume_info(root_path: &Path, disks: &Disks) -> (String, u64, u64) {
    // Convert root_path to absolute path
    #[cfg(windows)]
    let mut abs_root_path = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    #[cfg(not(windows))]
    let abs_root_path = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    // Remove the "\\?\" prefix on Windows
    #[cfg(windows)]
    {
        abs_root_path =
            std::path::PathBuf::from(abs_root_path.to_string_lossy().replacen(r"\\?\", "", 1));
    }

    // Collect and sort disks by the depth of their mount points (in descending order)
    let mut disks: Vec<_> = disks.iter().collect();
    disks.sort_by_key(|disk| Reverse(disk.mount_point().components().count()));

    // Find the first matching disk
    for disk in disks {
        let mount_point = disk.mount_point();

        if abs_root_path.starts_with(mount_point) {
            let volume_path = mount_point.to_string_lossy().to_string();
            let volume_size = disk.total_space();
            let free_space = disk.available_space();
            return (volume_path, volume_size, free_space);
        }
    }

    // If no matching disk is found, return defaults
    (
        "/".to_string(),
        0, // volume_size
        0, // free_space
    )
}

/// Recursively traverses the directory and outputs XML.
fn traverse_directory_to_xml(
    path: &Path,
    is_root: bool,
    root_dev: u64,
    apparent_size_flag: bool,
    cross_mount_points: bool,
    include_zero_files: bool,
    include_empty_folders: bool,
    visited_inodes: &mut HashSet<u64>,
) -> io::Result<()> {
    // Get metadata of the current directory
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "[gpscan] Error: Failed to access metadata for '{}': {}",
                path.display(),
                e
            );
            return Ok(());
        }
    };

    // Check if the current directory is on a different filesystem
    if !cross_mount_points {
        let current_dev = get_device_id(&metadata);

        if current_dev != root_dev {
            eprintln!(
                "[gpscan] Skipping directory on different filesystem: {} (root: {}, current: {})",
                path.display(),
                root_dev,
                current_dev
            );
            return Ok(());
        }
    }

    // Get file times
    let (created, modified, accessed) = get_file_times(&metadata);

    // Get directory name
    let name = if is_root {
        path.display().to_string()
    } else {
        path.file_name()
            .unwrap_or_else(|| path.as_os_str())
            .to_string_lossy()
            .to_string()
    };

    // Read directory entries and count items
    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(read_dir) => read_dir
            .filter_map(|entry| match entry {
                Ok(e) => Some(e),
                Err(e) => {
                    eprintln!(
                        "[gpscan] Error: Failed to read directory entry in '{}': {}",
                        path.display(),
                        e
                    );
                    None
                }
            })
            .collect(),
        Err(e) => {
            eprintln!(
                "[gpscan] Error: Failed to read directory '{}': {}",
                path.display(),
                e
            );
            return Ok(());
        }
    };

    // Check if the folder is empty and should be skipped
    if entries.is_empty() && !include_empty_folders {
        eprintln!("[gpscan] Skipping empty folder: {}", path.display());
        return Ok(());
    }

    // Output Folder tag
    println!(
        r#"<Folder name="{}" created="{}" modified="{}" accessed="{}">"#,
        xml_escape(&name),
        created,
        modified,
        accessed
    );

    // Iterate over directory entries
    for entry in entries {
        let entry_path = entry.path();

        // Get metadata of the entry
        let entry_metadata = match fs::symlink_metadata(&entry_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "[gpscan] Error: Failed to access metadata for '{}': {}",
                    entry_path.display(),
                    e
                );
                continue;
            }
        };

        let file_type = entry_metadata.file_type();

        if file_type.is_symlink() {
            // Skip symbolic links
            eprintln!("[gpscan] Skipping symbolic link: {}", entry_path.display());
            continue;
        } else if file_type.is_dir() {
            // Recursively traverse directories
            traverse_directory_to_xml(
                &entry_path,
                false,
                root_dev,
                apparent_size_flag,
                cross_mount_points,
                include_zero_files,
                include_empty_folders,
                visited_inodes,
            )?;
        } else if file_type.is_file() {
            // Process file entries
            process_file_entry(
                &entry_path,
                &entry_metadata,
                include_zero_files,
                apparent_size_flag,
                visited_inodes,
            );
        } else {
            // Handle other file types
            eprintln!("[gpscan] Unknown file type: {}", entry_path.display());
        }
    }

    // Close Folder tag
    println!("</Folder>");
    Ok(())
}

fn try_bytes_from_path(path: &Path, apparent_size_flag: bool) -> u64 {
    match path.symlink_metadata() {
        #[cfg(any(target_os = "freebsd", target_os = "linux"))]
        Ok(metadata) => {
            if apparent_size_flag {
                metadata.st_size() as u64
            } else {
                metadata.st_blocks() as u64 * 512
            }
        }
        #[cfg(target_os = "macos")]
        Ok(metadata) => {
            if apparent_size_flag {
                metadata.size() as u64
            } else {
                metadata.blocks() as u64 * 512
            }
        }
        #[cfg(target_os = "windows")]
        Ok(metadata) => {
            if apparent_size_flag {
                metadata.len()
            } else {
                // Note: On Windows, the physical size is not available.
                eprintln!(
                    "[gpscan] Warning: Using logical volume size. Physical size is not available on Windows."
                );
                metadata.len()
            }
        }
        Err(err) => {
            eprintln!(
                "[gpscan] Error: Failed to access metadata for '{}': {} ({:?})",
                path.display(),
                err,
                err.kind()
            );
            0
        }
    }
}

/// Processes a file entry and outputs XML.
fn process_file_entry(
    path: &Path,
    metadata: &Metadata,
    include_zero_files: bool,
    apparent_size_flag: bool,
    visited_inodes: &mut HashSet<u64>,
) {
    // Get inode number
    let inode = get_inode_number(metadata);

    // Skip if the file is a hard link
    if visited_inodes.contains(&inode) {
        eprintln!("[gpscan] Skipping hard link file: {}", path.display());
        return;
    }

    // Add inode number to the set of visited inodes
    visited_inodes.insert(inode);

    // Get file name
    let name = path
        .file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy()
        .to_string();

    // Get physical file size
    let size = try_bytes_from_path(path, apparent_size_flag);

    // Skip zero-byte files if the `include_zero_files` option is not set
    if size == 0 && !include_zero_files {
        eprintln!("[gpscan] Skipping zero-byte file: {}", path.display());
        return;
    }

    // Get file times
    let (created, modified, accessed) = get_file_times(metadata);

    // Output File tag
    println!(
        r#"<File name="{}" size="{}" created="{}" modified="{}" accessed="{}" />"#,
        xml_escape(&name),
        size,
        created,
        modified,
        accessed
    );
}

/// Retrieves creation, modification, and access times from metadata.
fn get_file_times(metadata: &Metadata) -> (String, String, String) {
    let format_time = |sys_time: Result<SystemTime, std::io::Error>| match sys_time {
        Ok(t) => {
            let datetime: DateTime<Utc> = t.into();
            datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }
        Err(_) => "1970-01-01T00:00:00Z".to_string(),
    };

    let created = format_time(metadata.created());
    let modified = format_time(metadata.modified());
    let accessed = format_time(metadata.accessed());

    (created, modified, accessed)
}
