// External crates
use chrono::{DateTime, Utc};
use clap::{Arg, ArgMatches, Command};
use sysinfo::Disks;

// Standard library imports
use std::fs::{self, Metadata};
use std::io;
use std::os::unix::fs::MetadataExt; // For accessing device IDs
use std::path::Path;
use std::time::SystemTime;

// Constants representing GrandPerspective version information
const GRANDPERSPECTIVE_APP_VERSION: &str = "4";
const GRANDPERSPECTIVE_FORMAT_VERSION: &str = "7";

/// Entry point of the program.
fn main() -> io::Result<()> {
    let matches = parse_args();
    run(matches)
}

/// Parses command-line arguments using clap.
fn parse_args() -> ArgMatches {
    let bold_underline = "\x1b[1;4m";
    let bold = "\x1b[1m";
    let reset = "\x1b[0m";

    Command::new("gpscan")
        .version(clap::crate_version!())
        .about(&format!(
            "\n\n{}Program:{} {}gpscan{} (GrandPerspective XML Scan Dump)\n\
            Version: {}\n\
            Source:  https://github.com/kojix2/gpscan",
            bold_underline,
            reset,
            bold,
            reset,
            clap::crate_version!()
        ))
        .arg(
            Arg::new("directory")
                .help("The directory to scan (required)")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("mounts")
                .short('m')
                .long("mounts")
                .help("Cross filesystem boundaries during scan [false]")
                .num_args(0),
        )
        .arg_required_else_help(true)
        .get_matches()
}

/// Runs the main logic of the program.
fn run(matches: ArgMatches) -> io::Result<()> {
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

    // Determine whether to cross filesystem boundaries
    let cross_mount_points = matches.contains_id("mounts");

    // Get the device ID of the root directory
    let root_metadata = fs::metadata(root_path)?;
    let root_dev = root_metadata.dev();

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(root_path, &disks);

    // Output XML header
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

    // Start traversing the directory
    traverse_directory_to_xml(root_path, true, root_dev, cross_mount_points)?;

    // Close XML tags
    println!("</ScanInfo>");
    println!("</GrandPerspectiveScanDump>");

    Ok(())
}

/// Retrieves volume information for the given path.
fn get_volume_info(root_path: &Path, disks: &Disks) -> (String, u64, u64) {
    // Convert root_path to absolute path
    let abs_root_path = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    // Find the disk that contains the root_path
    for disk in disks.iter() {
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
    cross_mount_points: bool,
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

    // Get device ID of the current directory
    let current_dev = metadata.dev();

    // Check if we should skip directories on different filesystems
    if !cross_mount_points && current_dev != root_dev {
        eprintln!(
            "[gpscan] Skipping directory on different filesystem: {} (root: {}, current: {})",
            path.display(),
            root_dev,
            current_dev
        );
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

    // Output Folder tag
    println!(
        r#"<Folder name="{}" created="{}" modified="{}" accessed="{}">"#,
        xml_escape(&name),
        created,
        modified,
        accessed
    );

    // Read directory entries
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "[gpscan] Error: Failed to read directory '{}': {}",
                path.display(),
                e
            );
            println!("</Folder>");
            return Ok(());
        }
    };

    // Iterate over directory entries
    for entry in entries {
        match entry {
            Ok(entry) => {
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
                    traverse_directory_to_xml(&entry_path, false, root_dev, cross_mount_points)?;
                } else if file_type.is_file() {
                    // Process file entries
                    process_file_entry(&entry_path, &entry_metadata);
                } else {
                    // Handle other file types
                    eprintln!("[gpscan] Unknown file type: {}", entry_path.display());
                }
            }
            Err(e) => {
                eprintln!("[gpscan] Error: reading directory entry: {}", e);
            }
        }
    }

    // Close Folder tag
    println!("</Folder>");
    Ok(())
}

/// Processes a file entry and outputs XML.
fn process_file_entry(path: &Path, metadata: &Metadata) {
    // Get file name
    let name = path
        .file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy()
        .to_string();

    // Get file size
    let size = metadata.len();

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

/// Escapes special characters for XML output.
fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            c if c.is_control() || c == '\u{FFFD}' => format!("&#x{:X};", c as u32),
            c => c.to_string(),
        })
        .collect()
}
