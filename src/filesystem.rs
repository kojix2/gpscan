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
use std::io::{self, Write};
use std::path::Path;

use crate::compression::create_compressed_writer_with_level;
use crate::options::Options;
use crate::platform::MetadataExtOps;
use crate::scan::traverse_directory_to_xml;
use crate::volume::get_volume_info;
use crate::xml_output::{output_xml_header, TAG_GRANDPERSPECTIVE_SCAN_DUMP, TAG_SCAN_INFO};

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
    let root_metadata = fs::metadata(root_path)?;
    let root_dev = root_metadata.device_id();

    // Create Disks instance and refresh disk list
    let disks = Disks::new_with_refreshed_list();

    // Get volume information
    let (volume_path, volume_size, free_space) = get_volume_info(root_path, &disks);

    // Create a write handle with compression support
    let handle: Box<dyn Write> = match &option.output_filename {
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
            let file = fs::File::create(filename)?;
            create_compressed_writer_with_level(
                file,
                option.compression_type,
                option.compression_level,
            )?
        }
        None => create_compressed_writer_with_level(
            io::stdout(),
            option.compression_type,
            option.compression_level,
        )?,
    };

    let mut writer = Writer::new_with_indent(handle, b' ', 0);

    // Output the XML header and start tag
    output_xml_header(&mut writer)?;

    // Output the scan information
    let scan_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut scan_info = BytesStart::new(TAG_SCAN_INFO);
    scan_info.push_attribute((
        "volumePath",
        quick_xml::escape::escape(&volume_path).as_ref(),
    ));
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

    // Add final newline for consistency with quick-xml Writer (always uses \n)
    writer
        .get_mut()
        .write_all(b"\n")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}
