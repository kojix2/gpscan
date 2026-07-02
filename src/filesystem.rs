// External crates
use chrono::Utc;
use clap::ArgMatches;
use log::error;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;
use sysinfo::Disks;

// Standard library imports
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::compression::create_compressed_writer_with_level;
use crate::options::Options;
use crate::platform::MetadataExtOps;
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
    let root_dev = root_metadata.device_id();

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(&root_path_abs, &disks);
    let volume_root = Path::new(&volume_path);
    let root_label = root_label_for(&root_path_abs, volume_root);

    // Create a write handle with compression support
    let (handle, output_path_to_skip): (Box<dyn Write>, Option<PathBuf>) = match &option
        .output_filename
    {
        Some(filename) => {
            // Validate that the provided output is not a directory-like path
            // Note: We only check obvious cases (ends_with separator or path exists and is dir)
            let path = Path::new(filename);
            if path.as_os_str().is_empty()
                || path.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR)
            {
                let msg = format!("Output path looks like a directory: {}", filename);
                error!("{}", msg);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
            }
            if path.exists() && path.is_dir() {
                let msg = format!("Output path is a directory: {}", filename);
                error!("{}", msg);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
            }
            // Overwrite confirmation when file exists
            if path.exists() && path.is_file() && !option.force_overwrite {
                let stdout_is_tty = io::stdout().is_terminal();
                let stdin_is_tty = io::stdin().is_terminal();
                if stdout_is_tty && stdin_is_tty {
                    eprint!("[gpscan] [WARN] '{}' exists. Overwrite? [y/N]: ", filename);
                    io::stderr().flush().ok();
                    let mut buf = String::new();
                    io::stdin().read_line(&mut buf).ok();
                    let ans = buf.trim().to_lowercase();
                    if ans != "y" && ans != "yes" {
                        let msg = "Operation cancelled by user".to_string();
                        error!("{}", msg);
                        return Err(io::Error::other(msg));
                    }
                } else {
                    let msg = format!(
                        "Refusing to overwrite existing file without --force in non-interactive mode: {}",
                        filename
                    );
                    error!("{}", msg);
                    return Err(io::Error::other(msg));
                }
            }
            let file = fs::File::create(filename)?;
            let output_path_to_skip = fs::canonicalize(filename).ok();
            let handle = create_compressed_writer_with_level(
                file,
                option.compression_type,
                option.compression_level,
            )?;
            (handle, output_path_to_skip)
        }
        None => (
            create_compressed_writer_with_level(
                io::stdout(),
                option.compression_type,
                option.compression_level,
            )?,
            None,
        ),
    };

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

    // Start traversing the directory with new options
    let traversal_config = TraversalConfig {
        root_label: &root_label,
        root_dev,
        options: &option,
        output_path_to_skip: output_path_to_skip.as_deref(),
    };

    traverse_directory_to_xml_with_config(
        &root_path_abs,
        true,
        &traversal_config,
        &mut visited_inodes,
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

    Ok(())
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
