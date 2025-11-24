// External crates
use log::{error, info, warn};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;

// Standard library imports
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::Path;

use crate::options::Options;
use crate::platform::MetadataExtOps;
use crate::xml_output::{get_file_times, TAG_FILE, TAG_FOLDER};

/// Recursively traverses the directory and outputs XML.
pub fn traverse_directory_to_xml<W: Write>(
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
            .unwrap_or(path.as_os_str())
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
    folder_tag.push_attribute(("name", quick_xml::escape::escape(&name).as_ref()));
    folder_tag.push_attribute(("created", created.as_str()));
    folder_tag.push_attribute(("modified", modified.as_str()));
    folder_tag.push_attribute(("accessed", accessed.as_str()));
    writer
        .write_event(Event::Start(folder_tag))
        .map_err(io::Error::other)?;

    // GrandPerspective compliance: output File elements before Folder elements (two-pass classification)
    let mut file_entries = Vec::new();
    let mut dir_entries = Vec::new();

    for entry in entries {
        let entry_path = entry.path();
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
        let ft = entry_metadata.file_type();
        if ft.is_symlink() {
            info!("Skipping symbolic link: {}", entry_path.display());
            continue;
        }
        if ft.is_file() {
            file_entries.push((entry_path, entry_metadata));
        } else if ft.is_dir() {
            dir_entries.push(entry_path);
        } else {
            warn!("Unknown file type: {}", entry_path.display());
        }
    }

    // Files first
    for (entry_path, entry_metadata) in file_entries {
        process_file_entry(
            &entry_path,
            &entry_metadata,
            options,
            visited_inodes,
            writer,
        )?;
    }
    // Then directories (depth-first behavior preserved; only sibling ordering changes)
    for entry_path in dir_entries {
        traverse_directory_to_xml(
            &entry_path,
            false,
            root_dev,
            options,
            visited_inodes,
            writer,
        )?;
    }

    // Close Folder tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_FOLDER)))
        .map_err(io::Error::other)?;
    Ok(())
}

/// Processes a file entry and outputs XML.
pub fn process_file_entry<W: Write>(
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
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_string();

    // Get file size (logical or physical depending on options.apparent_size)
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
    file_tag.push_attribute(("name", quick_xml::escape::escape(&name).as_ref()));
    file_tag.push_attribute(("size", size.to_string().as_str()));
    file_tag.push_attribute(("created", created.as_str()));
    file_tag.push_attribute(("modified", modified.as_str()));
    file_tag.push_attribute(("accessed", accessed.as_str()));
    writer
        .write_event(Event::Empty(file_tag))
        .map_err(io::Error::other)?;

    Ok(())
}

/// Reads the contents of a directory and returns a vector of directory entries.
pub fn read_directory(path: &Path) -> io::Result<Vec<fs::DirEntry>> {
    match fs::read_dir(path) {
        Ok(read_dir) => read_dir.collect::<Result<Vec<_>, io::Error>>(),
        Err(e) => {
            error!("Failed to read directory '{}': {}", path.display(), e);
            Err(e)
        }
    }
}

pub fn get_metadata(path: &Path) -> io::Result<Metadata> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(e) => {
            error!("Failed to access metadata for '{}': {}", path.display(), e);
            Err(e)
        }
    }
}
