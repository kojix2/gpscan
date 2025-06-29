#![cfg_attr(windows, feature(windows_by_handle))] // volume_serial_number

pub mod args;
pub mod compression;
pub mod filesystem;
pub mod options;
pub mod platform;
pub mod scan;
pub mod volume;
pub mod xml_output;

pub use args::parse_args;
pub use filesystem::run;

// Re-export core functionality for library use
pub use compression::{create_compressed_writer, CompressionType};
pub use options::Options;
pub use scan::{process_file_entry, traverse_directory_to_xml};
pub use volume::get_volume_info;
pub use xml_output::{get_file_times, output_xml_header};
