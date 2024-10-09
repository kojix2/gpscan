#![cfg_attr(windows, feature(windows_by_handle))] // volume_serial_number

pub mod args;
pub mod filesystem;
pub mod xml;

pub use args::parse_args;
pub use filesystem::run;
pub use xml::xml_escape;
