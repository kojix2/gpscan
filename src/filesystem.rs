// External crates
use chrono::Utc;
use clap::ArgMatches;
use log::error;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;
use sysinfo::Disks;

// Standard library imports
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::compression::create_finishable_writer_with_level;
use crate::options::Options;
use crate::platform::{path_device_id, replace_file};
use crate::scan::{traverse_directory_to_xml_with_config, TraversalConfig};
use crate::volume::get_volume_info;
use crate::xml_output::{
    output_xml_header, sanitize_for_xml, TAG_GRANDPERSPECTIVE_SCAN_DUMP, TAG_SCAN_INFO,
};

/// Runs the main logic of the program.
pub fn run(matches: ArgMatches) -> io::Result<()> {
    // Get the directory path from arguments
    let directory = matches
        .get_one::<String>("directory")
        .expect("Directory path is required")
        .as_str();

    let root_path = Path::new(directory);
    let root_path_abs = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    // Check if the provided path exists
    if !root_path.exists() {
        let msg = format!("The specified path does not exist: {}", root_path.display());
        error!("{}", msg);
        return Err(io::Error::new(io::ErrorKind::NotFound, msg));
    }

    // Check if the provided path is a directory
    if !root_path.is_dir() {
        let msg = format!(
            "The specified path is not a directory: {}",
            root_path.display()
        );
        error!("{}", msg);
        return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
    }

    // Get option values
    let option = Options::from_matches(&matches);

    // Get the device ID of the root directory
    let root_metadata = fs::metadata(&root_path_abs)?;
    let root_dev = path_device_id(&root_path_abs, &root_metadata)?;

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(&root_path_abs, &disks);
    let volume_root = Path::new(&volume_path);
    let root_label = root_label_for(&root_path_abs, volume_root);

    let prepared_output = prepare_output(&option)?;
    let output_paths_to_skip = prepared_output.paths_to_skip.clone();
    let mut output_destination = prepared_output.destination;
    let handle = create_finishable_writer_with_level(
        prepared_output.handle,
        option.compression_type,
        option.compression_level,
    );

    let mut writer = Writer::new_with_indent(handle, b' ', 0);

    // Output the XML header and start tag
    output_xml_header(&mut writer)?;

    // Output the scan information
    let scan_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut scan_info = BytesStart::new(TAG_SCAN_INFO);
    let sanitized_volume_path = sanitize_for_xml(&volume_path);
    scan_info.push_attribute(("volumePath", sanitized_volume_path.as_str()));
    scan_info.push_attribute(("volumeSize", volume_size.to_string().as_str()));
    scan_info.push_attribute(("freeSpace", free_space.to_string().as_str()));
    scan_info.push_attribute(("scanTime", scan_time.to_string().as_str()));
    let measure = if option.apparent_size {
        "logical"
    } else {
        "physical"
    };
    scan_info.push_attribute(("fileSizeMeasure", measure));
    writer
        .write_event(Event::Start(scan_info))
        .map_err(io::Error::other)?;

    // Create a set to store visited inodes
    let mut visited_inodes = HashSet::new();
    let mut visited_dirs = HashSet::new();

    // Start traversing the directory with new options
    let traversal_config = TraversalConfig {
        root_label: &root_label,
        root_dev,
        options: &option,
        output_paths_to_skip: &output_paths_to_skip,
    };

    traverse_directory_to_xml_with_config(
        &root_path_abs,
        true,
        &traversal_config,
        &mut visited_inodes,
        &mut visited_dirs,
        &mut writer,
    )?;

    // </ScanInfo> tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_SCAN_INFO)))
        .map_err(io::Error::other)?;
    // </GrandPerspectiveScanDump> tag
    writer
        .write_event(Event::End(BytesEnd::new(TAG_GRANDPERSPECTIVE_SCAN_DUMP)))
        .map_err(io::Error::other)?;

    // Add final newline for consistency with quick-xml Writer (always uses \n)
    writer
        .get_mut()
        .write_all(b"\n")
        .map_err(io::Error::other)?;

    let handle = writer.into_inner();
    let mut raw_handle = handle.finish()?;
    raw_handle.flush()?;
    drop(raw_handle);
    output_destination.commit()?;

    Ok(())
}

struct PreparedOutput {
    handle: Box<dyn Write>,
    destination: OutputDestination,
    paths_to_skip: Vec<PathBuf>,
}

enum OutputDestination {
    Stdout,
    AtomicFile(AtomicFileOutput),
}

impl OutputDestination {
    fn commit(&mut self) -> io::Result<()> {
        match self {
            OutputDestination::Stdout => Ok(()),
            OutputDestination::AtomicFile(output) => output.commit(),
        }
    }
}

struct AtomicFileOutput {
    target_path: PathBuf,
    temp_path: PathBuf,
    committed: bool,
}

impl AtomicFileOutput {
    fn commit(&mut self) -> io::Result<()> {
        replace_file(&self.temp_path, &self.target_path)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for AtomicFileOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

fn prepare_output(option: &Options) -> io::Result<PreparedOutput> {
    match &option.output_filename {
        Some(filename) => prepare_file_output(filename, option),
        None => Ok(PreparedOutput {
            handle: Box::new(io::stdout()),
            destination: OutputDestination::Stdout,
            paths_to_skip: Vec::new(),
        }),
    }
}

fn prepare_file_output(filename: &str, option: &Options) -> io::Result<PreparedOutput> {
    let target_path = Path::new(filename);
    validate_output_target(target_path, filename, option.force_overwrite)?;

    let mut paths_to_skip = Vec::new();
    if let Ok(path) = fs::canonicalize(target_path) {
        paths_to_skip.push(path);
    }

    let (file, temp_path) = create_temp_output_file(target_path)?;
    if let Ok(path) = fs::canonicalize(&temp_path) {
        paths_to_skip.push(path);
    }

    Ok(PreparedOutput {
        handle: Box::new(file),
        destination: OutputDestination::AtomicFile(AtomicFileOutput {
            target_path: target_path.to_path_buf(),
            temp_path,
            committed: false,
        }),
        paths_to_skip,
    })
}

fn validate_output_target(path: &Path, filename: &str, force_overwrite: bool) -> io::Result<()> {
    if path.as_os_str().is_empty() || path.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR) {
        let msg = format!("Output path looks like a directory: {}", filename);
        error!("{}", msg);
        return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
    }

    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                let msg = format!(
                    "Refusing to write output through symbolic link: {}",
                    filename
                );
                error!("{}", msg);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
            }
            if file_type.is_dir() {
                let msg = format!("Output path is a directory: {}", filename);
                error!("{}", msg);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
            }
            if !file_type.is_file() {
                let msg = format!("Output path is not a regular file: {}", filename);
                error!("{}", msg);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
            }
            confirm_overwrite_if_needed(filename, force_overwrite)?;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }

    Ok(())
}

fn confirm_overwrite_if_needed(filename: &str, force_overwrite: bool) -> io::Result<()> {
    if force_overwrite {
        return Ok(());
    }

    let stdout_is_tty = io::stdout().is_terminal();
    let stdin_is_tty = io::stdin().is_terminal();
    if stdout_is_tty && stdin_is_tty {
        eprint!("[gpscan] [WARN] '{}' exists. Overwrite? [y/N]: ", filename);
        io::stderr().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();
        let ans = buf.trim().to_lowercase();
        if ans == "y" || ans == "yes" {
            return Ok(());
        }

        let msg = "Operation cancelled by user".to_string();
        error!("{}", msg);
        Err(io::Error::other(msg))
    } else {
        let msg = format!(
            "Refusing to overwrite existing file without --force in non-interactive mode: {}",
            filename
        );
        error!("{}", msg);
        Err(io::Error::other(msg))
    }
}

fn create_temp_output_file(target_path: &Path) -> io::Result<(File, PathBuf)> {
    let parent = target_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = target_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Output path has no file name"))?
        .to_string_lossy();
    let pid = std::process::id();

    for attempt in 0..100 {
        let temp_path = parent.join(format!(".{}.tmp.{}.{}", file_name, pid, attempt));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((file, temp_path)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "Could not create a unique temporary output file for {}",
            target_path.display()
        ),
    ))
}

fn root_label_for(root_path_abs: &Path, volume_root: &Path) -> String {
    if let Ok(rel) = root_path_abs.strip_prefix(volume_root) {
        let label = rel
            .to_string_lossy()
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .to_string();

        if !label.is_empty() {
            return label;
        }

        return path_display_name(root_path_abs);
    }

    root_path_abs.display().to_string()
}

fn path_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn test_root_label_uses_relative_path_under_volume_root() {
        let root_path = Path::new("/Users/example/project");
        let volume_root = Path::new("/");

        assert_eq!(
            root_label_for(root_path, volume_root),
            "Users/example/project"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_root_label_uses_relative_path_under_volume_root() {
        let root_path = Path::new(r"C:\Users\example\project");
        let volume_root = Path::new(r"C:\");

        assert_eq!(
            root_label_for(root_path, volume_root),
            r"Users\example\project"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn test_root_label_uses_mount_point_name_for_volume_root() {
        let root_path = Path::new("/Volumes/Data");
        let volume_root = Path::new("/Volumes/Data");

        assert_eq!(root_label_for(root_path, volume_root), "Data");
    }

    #[cfg(windows)]
    #[test]
    fn test_root_label_uses_mount_point_name_for_volume_root() {
        let root_path = Path::new(r"C:\Data");
        let volume_root = Path::new(r"C:\Data");

        assert_eq!(root_label_for(root_path, volume_root), "Data");
    }

    #[cfg(not(windows))]
    #[test]
    fn test_root_label_uses_root_path_for_filesystem_root() {
        let root_path = Path::new("/");
        let volume_root = Path::new("/");

        assert_eq!(root_label_for(root_path, volume_root), "/");
    }

    #[cfg(windows)]
    #[test]
    fn test_root_label_uses_root_path_for_filesystem_root() {
        let root_path = Path::new(r"C:\");
        let volume_root = Path::new(r"C:\");

        assert_eq!(root_label_for(root_path, volume_root), r"C:\");
    }

    #[cfg(not(windows))]
    #[test]
    fn test_root_label_falls_back_to_absolute_path_outside_volume_root() {
        let root_path = Path::new("/other/path");
        let volume_root = Path::new("/Volumes/Data");

        assert_eq!(root_label_for(root_path, volume_root), "/other/path");
    }

    #[cfg(windows)]
    #[test]
    fn test_root_label_falls_back_to_absolute_path_outside_volume_root() {
        let root_path = Path::new(r"D:\other\path");
        let volume_root = Path::new(r"C:\Data");

        assert_eq!(root_label_for(root_path, volume_root), r"D:\other\path");
    }
}
