// External crates
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use log::{error, info, warn};
use quick_xml::escape::escape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;
use sysinfo::Disks;

// Standard library imports
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::Path;
use std::time::SystemTime;

use crate::platform::MetadataExtOps; // Ensure this trait is implemented for Metadata

// Constants for XML output
const GRANDPERSPECTIVE_APP_VERSION: &str = "4";
const GRANDPERSPECTIVE_FORMAT_VERSION: &str = "7";
const XML_VERSION: &str = "1.0";
const XML_ENCODING: &str = "UTF-8";
const DEFAULT_DATETIME: &str = "1970-01-01T00:00:00Z";
const TAG_SCAN_INFO: &str = "ScanInfo";
const TAG_GRANDPERSPECTIVE_SCAN_DUMP: &str = "GrandPerspectiveScanDump";
const TAG_FOLDER: &str = "Folder";
const TAG_FILE: &str = "File";

pub struct Options {
    apparent_size: bool,
    cross_mount_points: bool,
    include_zero_files: bool,
    include_empty_folders: bool,
}

impl Options {
    pub fn from_matches(matches: &ArgMatches) -> Self {
        Options {
            apparent_size: matches.get_flag("apparent-size"),
            cross_mount_points: matches.get_flag("mounts"),
            include_zero_files: matches.get_flag("include-zero-files"),
            include_empty_folders: matches.get_flag("include-empty-folders"),
        }
    }
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
        error!("The specified path does not exist: {}", root_path.display());
        std::process::exit(1); // Exit code 1 for non-existent path
    }

    // Check if the provided path is a directory
    if !root_path.is_dir() {
        error!(
            "The specified path is not a directory: {}",
            root_path.display()
        );
        std::process::exit(1); // Exit code 1 for invalid directory
    }

    // Get option values
    let option = Options::from_matches(&matches);

    // Get the device ID of the root directory
    let root_metadata = fs::metadata(root_path)?;
    let root_dev = root_metadata.device_id();

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(root_path, &disks);

    // Determine output destination
    let output = matches.get_one::<String>("output");

    // Create a write handle
    let handle: Box<dyn Write> = match output {
        Some(file) => {
            let file = fs::File::create(file)?;
            Box::new(file)
        }
        None => Box::new(io::stdout()),
    };

    let mut writer = Writer::new_with_indent(handle, b' ', 0);

    // Output the XML header and start tag
    output_xml_header(&mut writer)?;

    // Output the scan information
    let scan_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut scan_info = BytesStart::new(TAG_SCAN_INFO);
    scan_info.push_attribute(("volumePath", escape(&volume_path).as_ref()));
    scan_info.push_attribute(("volumeSize", volume_size.to_string().as_str()));
    scan_info.push_attribute(("freeSpace", free_space.to_string().as_str()));
    scan_info.push_attribute(("scanTime", scan_time.to_string().as_str()));
    scan_info.push_attribute(("fileSizeMeasure", "physical"));
    writer
        .write_event(Event::Start(scan_info))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Create a set to store visited inodes
    let mut visited_inodes = HashSet::new();

    // Start traversing the directory with new options
    traverse_directory_to_xml(
        root_path,
        true,
        root_dev,
        &option,
        &mut visited_inodes,
        &mut writer,
    )?;

    // </ScanInfo> tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_SCAN_INFO)))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    // </GrandPerspectiveScanDump> tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_GRANDPERSPECTIVE_SCAN_DUMP)))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

/// Reads the contents of a directory and returns a vector of directory entries.
fn read_directory(path: &Path) -> io::Result<Vec<fs::DirEntry>> {
    match fs::read_dir(path) {
        Ok(read_dir) => read_dir.collect::<Result<Vec<_>, io::Error>>(),
        Err(e) => {
            error!("Failed to read directory '{}': {}", path.display(), e);
            Err(e)
        }
    }
}

fn get_metadata(path: &Path) -> io::Result<Metadata> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(e) => {
            error!("Failed to access metadata for '{}': {}", path.display(), e);
            Err(e)
        }
    }
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

fn output_xml_header<W: Write>(writer: &mut Writer<W>) -> io::Result<()> {
    writer
        .write_event(Event::Decl(BytesDecl::new(
            XML_VERSION,
            Some(XML_ENCODING),
            None,
        )))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut root = BytesStart::new(TAG_GRANDPERSPECTIVE_SCAN_DUMP);
    root.push_attribute(("appVersion", GRANDPERSPECTIVE_APP_VERSION));
    root.push_attribute(("formatVersion", GRANDPERSPECTIVE_FORMAT_VERSION));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

/// Recursively traverses the directory and outputs XML.
fn traverse_directory_to_xml<W: Write>(
    path: &Path,
    is_root: bool,
    root_dev: u64,
    options: &Options,
    visited_inodes: &mut HashSet<u64>,
    writer: &mut Writer<W>,
) -> io::Result<()> {
    // Get metadata of the current directory
    let metadata = match get_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };

    // Check if the current directory is on a different filesystem
    if !options.cross_mount_points {
        let current_dev = metadata.device_id();

        if current_dev != root_dev {
            info!(
                "Skipping directory on different filesystem: {} (root: {}, current: {})",
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

    // Read directory entries
    let mut entries: Vec<_> = match read_directory(path) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    // Check if the folder is empty and should be skipped
    if entries.is_empty() && !options.include_empty_folders {
        info!("Skipping empty folder: {}", path.display());
        return Ok(());
    }

    // Sort entries by file name
    entries.sort_by(|a, b| {
        a.file_name()
            .to_string_lossy()
            .cmp(&b.file_name().to_string_lossy())
    });

    // Output Folder tag
    let mut folder_tag = BytesStart::new(TAG_FOLDER);
    folder_tag.push_attribute(("name", escape(&name).as_ref()));
    folder_tag.push_attribute(("created", created.as_str()));
    folder_tag.push_attribute(("modified", modified.as_str()));
    folder_tag.push_attribute(("accessed", accessed.as_str()));
    writer
        .write_event(Event::Start(folder_tag))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Iterate over directory entries
    for entry in entries {
        let entry_path = entry.path();

        // Get metadata of the entry
        let entry_metadata = match fs::symlink_metadata(&entry_path) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    "Failed to access metadata for '{}': {}",
                    entry_path.display(),
                    e
                );
                continue;
            }
        };

        let file_type = entry_metadata.file_type();

        if file_type.is_symlink() {
            // Skip symbolic links
            info!("Skipping symbolic link: {}", entry_path.display());
            continue;
        } else if file_type.is_dir() {
            // Recursively traverse directories
            traverse_directory_to_xml(
                &entry_path,
                false,
                root_dev,
                options,
                visited_inodes,
                writer,
            )?;
        } else if file_type.is_file() {
            // Process file entries
            process_file_entry(
                &entry_path,
                &entry_metadata,
                options,
                visited_inodes,
                writer,
            )?;
        } else {
            // Handle other file types
            warn!("Unknown file type: {}", entry_path.display());
        }
    }

    // Close Folder tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_FOLDER)))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

/// Processes a file entry and outputs XML.
fn process_file_entry<W: Write>(
    path: &Path,
    metadata: &Metadata,
    options: &Options,
    visited_inodes: &mut HashSet<u64>,
    writer: &mut Writer<W>,
) -> io::Result<()> {
    // Get inode number
    let inode = metadata.inode_number();

    // Skip if the file is a hard link
    if visited_inodes.contains(&inode) {
        info!("Skipping hard link file: {}", path.display());
        return Ok(());
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
    let size = metadata.file_size(options.apparent_size);

    // Skip zero-byte files if the `include_zero_files` option is not set
    if size == 0 && !options.include_zero_files {
        info!("Skipping zero-byte file: {}", path.display());
        return Ok(());
    }

    // Get file times
    let (created, modified, accessed) = get_file_times(metadata);

    // Output File tag
    let mut file_tag = BytesStart::new(TAG_FILE);
    file_tag.push_attribute(("name", escape(&name).as_ref()));
    file_tag.push_attribute(("size", size.to_string().as_str()));
    file_tag.push_attribute(("created", created.as_str()));
    file_tag.push_attribute(("modified", modified.as_str()));
    file_tag.push_attribute(("accessed", accessed.as_str()));
    writer
        .write_event(Event::Empty(file_tag))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

fn format_system_time(sys_time: Result<SystemTime, io::Error>) -> String {
    match sys_time {
        Ok(t) => {
            let datetime: DateTime<Utc> = t.into();
            datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }
        Err(_) => DEFAULT_DATETIME.to_string(),
    }
}

/// Retrieves creation, modification, and access times from metadata.
fn get_file_times(metadata: &Metadata) -> (String, String, String) {
    let created = format_system_time(metadata.created());
    let modified = format_system_time(metadata.modified());
    let accessed = format_system_time(metadata.accessed());

    (created, modified, accessed)
}
