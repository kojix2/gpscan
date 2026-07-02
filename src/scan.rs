// External crates
use log::{error, info, warn};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;

// Standard library imports
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::options::Options;
use crate::platform::{file_identity, path_identity, MetadataExtOps};
use crate::xml_output::{get_file_times, sanitize_for_xml, TAG_FILE, TAG_FOLDER};

pub(crate) struct TraversalConfig<'a> {
    pub(crate) root_label: &'a str,
    pub(crate) root_dev: Option<u64>,
    pub(crate) options: &'a Options,
    pub(crate) output_paths_to_skip: &'a [PathBuf],
}

/// Recursively traverses the directory and outputs XML.
pub fn traverse_directory_to_xml<W: Write>(
    path: &Path,
    is_root: bool,
    root_label: &str,
    root_dev: u64,
    options: &Options,
    visited_inodes: &mut HashSet<(u64, u64)>,
    writer: &mut Writer<W>,
) -> io::Result<()> {
    let config = TraversalConfig {
        root_label,
        root_dev: (root_dev != 0).then_some(root_dev),
        options,
        output_paths_to_skip: &[],
    };

    let mut visited_dirs = HashSet::new();
    let mut pending_folders = Vec::new();
    traverse_directory_to_xml_impl(
        path,
        is_root,
        &config,
        visited_inodes,
        &mut visited_dirs,
        &mut pending_folders,
        writer,
    )
    .map(|_| ())
}

pub(crate) fn traverse_directory_to_xml_with_config<W: Write>(
    path: &Path,
    is_root: bool,
    config: &TraversalConfig<'_>,
    visited_inodes: &mut HashSet<(u64, u64)>,
    visited_dirs: &mut HashSet<(u64, u64)>,
    writer: &mut Writer<W>,
) -> io::Result<()> {
    let mut pending_folders = Vec::new();
    traverse_directory_to_xml_impl(
        path,
        is_root,
        config,
        visited_inodes,
        visited_dirs,
        &mut pending_folders,
        writer,
    )
    .map(|_| ())
}

struct FolderFrame {
    name: String,
    created: String,
    modified: String,
    accessed: String,
    started: bool,
}

fn traverse_directory_to_xml_impl<W: Write>(
    path: &Path,
    is_root: bool,
    config: &TraversalConfig<'_>,
    visited_inodes: &mut HashSet<(u64, u64)>,
    visited_dirs: &mut HashSet<(u64, u64)>,
    pending_folders: &mut Vec<FolderFrame>,
    writer: &mut Writer<W>,
) -> io::Result<bool> {
    // Get metadata of the current directory (suppress internal log for root; main.rs will print it)
    let metadata = match get_metadata_impl(path, !is_root) {
        Ok(metadata) => metadata,
        Err(e) => {
            if is_root {
                return Err(e);
            }
            return Ok(false);
        }
    };

    let dir_identity = match path_identity(path, &metadata) {
        Ok(identity) => identity,
        Err(e) => {
            warn!("Failed to identify directory '{}': {}", path.display(), e);
            None
        }
    };
    let current_dev = dir_identity
        .map(|(dev, _)| dev)
        .or_else(|| nonzero_device_id(&metadata));

    // Check if the current directory is on a different filesystem
    if !config.options.cross_mount_points {
        if let (Some(root_dev), Some(current_dev)) = (config.root_dev, current_dev) {
            if current_dev != root_dev {
                info!(
                    "Skipping directory on different filesystem: {} (root: {}, current: {})",
                    path.display(),
                    root_dev,
                    current_dev
                );
                return Ok(false);
            }
        } else {
            info!(
                "Cannot determine filesystem boundary for directory: {}",
                path.display(),
            );
        }
    }

    if let Some(dir_key) = dir_identity {
        if visited_dirs.contains(&dir_key) {
            info!("Skipping already visited directory: {}", path.display());
            return Ok(false);
        }
        visited_dirs.insert(dir_key);
    }

    // Get file times
    let (created, modified, accessed) = get_file_times(&metadata);

    // Get directory name
    let name = if is_root {
        config.root_label.to_string()
    } else {
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .to_string()
    };

    // Read directory entries (suppress internal log for root; main.rs will print it)
    let mut entries: Vec<_> = match read_directory_impl(path, !is_root) {
        Ok(entries) => entries,
        Err(e) => {
            if is_root {
                return Err(e);
            }
            return Ok(false);
        }
    };

    // Sort entries by file name
    entries.sort_by(|a, b| {
        a.file_name()
            .to_string_lossy()
            .cmp(&b.file_name().to_string_lossy())
    });

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

    let frame_index = pending_folders.len();
    pending_folders.push(FolderFrame {
        name,
        created,
        modified,
        accessed,
        started: false,
    });

    let mut has_output_children = false;

    // Files first
    for (entry_path, entry_metadata) in file_entries {
        if should_skip_output_file(&entry_path, config.output_paths_to_skip) {
            info!("Skipping output file: {}", entry_path.display());
            continue;
        }

        if process_file_entry_impl(
            &entry_path,
            &entry_metadata,
            config.options,
            visited_inodes,
            Some(pending_folders),
            writer,
        )? {
            has_output_children = true;
        }
    }
    // Then directories (depth-first behavior preserved; only sibling ordering changes)
    for entry_path in dir_entries {
        if traverse_directory_to_xml_impl(
            &entry_path,
            false,
            config,
            visited_inodes,
            visited_dirs,
            pending_folders,
            writer,
        )? {
            has_output_children = true;
        }
    }

    if !has_output_children && (is_root || config.options.include_empty_folders) {
        start_pending_folders(pending_folders, writer)?;
        has_output_children = true;
    }

    let folder_started = pending_folders[frame_index].started;

    if folder_started {
        writer
            .write_event(Event::End(BytesEnd::new(TAG_FOLDER)))
            .map_err(io::Error::other)?;
    }

    pending_folders.pop();

    if !has_output_children {
        info!("Skipping empty folder: {}", path.display());
    }

    Ok(has_output_children)
}

/// Processes a file entry and outputs XML.
pub fn process_file_entry<W: Write>(
    path: &Path,
    metadata: &Metadata,
    options: &Options,
    visited_inodes: &mut HashSet<(u64, u64)>,
    writer: &mut Writer<W>,
) -> io::Result<()> {
    process_file_entry_impl(path, metadata, options, visited_inodes, None, writer).map(|_| ())
}

fn process_file_entry_impl<W: Write>(
    path: &Path,
    metadata: &Metadata,
    options: &Options,
    visited_inodes: &mut HashSet<(u64, u64)>,
    pending_folders: Option<&mut Vec<FolderFrame>>,
    writer: &mut Writer<W>,
) -> io::Result<bool> {
    // Get file size (logical or physical depending on options.apparent_size)
    let size = metadata.file_size(options.apparent_size);

    // Skip zero-byte files if the `include_zero_files` option is not set
    if size == 0 && !options.include_zero_files {
        info!("Skipping zero-byte file: {}", path.display());
        return Ok(false);
    }

    // Physical size mode avoids counting the same disk blocks multiple times.
    // Apparent size mode represents the logical path tree, so every hard link path is emitted.
    if !options.apparent_size {
        if let Some(file_key) = file_identity(path, metadata)? {
            // Skip if the file is a hard link
            if visited_inodes.contains(&file_key) {
                info!("Skipping hard link file: {}", path.display());
                return Ok(false);
            }

            // Add inode number to the set of visited inodes
            visited_inodes.insert(file_key);
        }
    }

    // Get file name
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_string();

    // Get file times
    let (created, modified, accessed) = get_file_times(metadata);

    if let Some(pending_folders) = pending_folders {
        start_pending_folders(pending_folders, writer)?;
    }

    // Output File tag
    let mut file_tag = BytesStart::new(TAG_FILE);
    let sanitized_name = sanitize_for_xml(&name);
    file_tag.push_attribute(("name", sanitized_name.as_str()));
    file_tag.push_attribute(("size", size.to_string().as_str()));
    file_tag.push_attribute(("created", created.as_str()));
    file_tag.push_attribute(("modified", modified.as_str()));
    file_tag.push_attribute(("accessed", accessed.as_str()));
    writer
        .write_event(Event::Empty(file_tag))
        .map_err(io::Error::other)?;

    Ok(true)
}

fn start_pending_folders<W: Write>(
    pending_folders: &mut [FolderFrame],
    writer: &mut Writer<W>,
) -> io::Result<()> {
    for frame in pending_folders {
        if frame.started {
            continue;
        }

        let mut folder_tag = BytesStart::new(TAG_FOLDER);
        let sanitized_name = sanitize_for_xml(&frame.name);
        folder_tag.push_attribute(("name", sanitized_name.as_str()));
        folder_tag.push_attribute(("created", frame.created.as_str()));
        folder_tag.push_attribute(("modified", frame.modified.as_str()));
        folder_tag.push_attribute(("accessed", frame.accessed.as_str()));
        writer
            .write_event(Event::Start(folder_tag))
            .map_err(io::Error::other)?;
        frame.started = true;
    }

    Ok(())
}

fn should_skip_output_file(path: &Path, output_paths_to_skip: &[PathBuf]) -> bool {
    fs::canonicalize(path)
        .ok()
        .is_some_and(|canonical_path| output_paths_to_skip.contains(&canonical_path))
}

fn nonzero_device_id(metadata: &Metadata) -> Option<u64> {
    let device_id = metadata.device_id();
    (device_id != 0).then_some(device_id)
}

/// Reads the contents of a directory and returns a vector of directory entries.
/// Set `log_error` to false when the caller will handle and report the error itself.
pub fn read_directory(path: &Path) -> io::Result<Vec<fs::DirEntry>> {
    read_directory_impl(path, true)
}

fn read_directory_impl(path: &Path, log_error: bool) -> io::Result<Vec<fs::DirEntry>> {
    match fs::read_dir(path) {
        Ok(read_dir) => {
            let mut entries = Vec::new();
            for entry in read_dir {
                match entry {
                    Ok(entry) => entries.push(entry),
                    Err(e) => warn!(
                        "Failed to read directory entry in '{}': {}",
                        path.display(),
                        e
                    ),
                }
            }
            Ok(entries)
        }
        Err(e) => {
            if log_error {
                error!("Failed to read directory '{}': {}", path.display(), e);
            }
            Err(e)
        }
    }
}

pub fn get_metadata(path: &Path) -> io::Result<Metadata> {
    get_metadata_impl(path, true)
}

fn get_metadata_impl(path: &Path, log_error: bool) -> io::Result<Metadata> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(e) => {
            if log_error {
                error!("Failed to access metadata for '{}': {}", path.display(), e);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;
    use std::fs::File;
    use tempdir::TempDir;

    #[test]
    fn test_traversal_skips_already_visited_directory_identity() {
        let temp_dir = TempDir::new("gpscan_visited_dir").expect("Failed to create temp dir");
        let root_path = temp_dir.path();
        let mut file = File::create(root_path.join("file.txt")).expect("Failed to create file");
        writeln!(file, "content").expect("Failed to write file");

        let options = Options::default();
        let metadata = fs::metadata(root_path).expect("Failed to read root metadata");
        let config = TraversalConfig {
            root_label: "gpscan_visited_dir",
            root_dev: nonzero_device_id(&metadata),
            options: &options,
            output_paths_to_skip: &[],
        };
        let mut visited_inodes = HashSet::new();
        let mut visited_dirs = HashSet::new();
        let mut pending_folders = Vec::new();
        let mut first_output = Vec::new();
        let mut first_writer = Writer::new(&mut first_output);

        assert!(traverse_directory_to_xml_impl(
            root_path,
            true,
            &config,
            &mut visited_inodes,
            &mut visited_dirs,
            &mut pending_folders,
            &mut first_writer,
        )
        .expect("First traversal failed"));

        let mut pending_folders = Vec::new();
        let mut second_output = Vec::new();
        let mut second_writer = Writer::new(&mut second_output);
        assert!(
            !traverse_directory_to_xml_impl(
                root_path,
                false,
                &config,
                &mut visited_inodes,
                &mut visited_dirs,
                &mut pending_folders,
                &mut second_writer,
            )
            .expect("Second traversal failed"),
            "Expected repeat visit to the same directory identity to be skipped"
        );
        assert!(
            second_output.is_empty(),
            "Skipped directory should not emit XML"
        );
    }
}
