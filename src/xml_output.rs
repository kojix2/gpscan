// External crates
use chrono::{DateTime, Utc};
use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::writer::Writer;

// Standard library imports
use std::fs::Metadata;
use std::io::{self, Write};
use std::time::SystemTime;

// Constants for XML output
pub const GRANDPERSPECTIVE_APP_VERSION: &str = "4";
pub const GRANDPERSPECTIVE_FORMAT_VERSION: &str = "7";
pub const XML_VERSION: &str = "1.0";
pub const XML_ENCODING: &str = "UTF-8";
pub const DEFAULT_DATETIME: &str = "1970-01-01T00:00:00Z";
pub const TAG_SCAN_INFO: &str = "ScanInfo";
pub const TAG_GRANDPERSPECTIVE_SCAN_DUMP: &str = "GrandPerspectiveScanDump";
pub const TAG_FOLDER: &str = "Folder";
pub const TAG_FILE: &str = "File";

pub fn output_xml_header<W: Write>(writer: &mut Writer<W>) -> io::Result<()> {
    writer
        .write_event(Event::Decl(BytesDecl::new(
            XML_VERSION,
            Some(XML_ENCODING),
            None,
        )))
        .map_err(io::Error::other)?;
    let mut root = BytesStart::new(TAG_GRANDPERSPECTIVE_SCAN_DUMP);
    root.push_attribute(("appVersion", GRANDPERSPECTIVE_APP_VERSION));
    root.push_attribute(("formatVersion", GRANDPERSPECTIVE_FORMAT_VERSION));
    writer
        .write_event(Event::Start(root))
        .map_err(io::Error::other)?;
    Ok(())
}

pub fn format_system_time(sys_time: Result<SystemTime, io::Error>) -> String {
    match sys_time {
        Ok(t) => {
            let datetime: DateTime<Utc> = t.into();
            datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }
        Err(_) => DEFAULT_DATETIME.to_string(),
    }
}

/// Retrieves creation, modification, and access times from metadata.
pub fn get_file_times(metadata: &Metadata) -> (String, String, String) {
    let created = format_system_time(metadata.created());
    let modified = format_system_time(metadata.modified());
    let accessed = format_system_time(metadata.accessed());

    (created, modified, accessed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_format_system_time_valid() {
        let system_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1609459200); // 2021-01-01 00:00:00 UTC
        let result = format_system_time(Ok(system_time));
        assert_eq!(result, "2021-01-01T00:00:00Z");
    }

    #[test]
    fn test_format_system_time_error() {
        let result = format_system_time(Err(io::Error::other("test error")));
        assert_eq!(result, DEFAULT_DATETIME);
    }

    #[test]
    fn test_output_xml_header() {
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut buffer);

        let result = output_xml_header(&mut writer);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(output.contains(r#"<GrandPerspectiveScanDump"#));
        assert!(output.contains(r#"appVersion="4""#));
        assert!(output.contains(r#"formatVersion="7""#));
    }

    #[test]
    fn test_constants() {
        assert_eq!(GRANDPERSPECTIVE_APP_VERSION, "4");
        assert_eq!(GRANDPERSPECTIVE_FORMAT_VERSION, "7");
        assert_eq!(XML_VERSION, "1.0");
        assert_eq!(XML_ENCODING, "UTF-8");
        assert_eq!(DEFAULT_DATETIME, "1970-01-01T00:00:00Z");
        assert_eq!(TAG_SCAN_INFO, "ScanInfo");
        assert_eq!(TAG_GRANDPERSPECTIVE_SCAN_DUMP, "GrandPerspectiveScanDump");
        assert_eq!(TAG_FOLDER, "Folder");
        assert_eq!(TAG_FILE, "File");
    }
}
