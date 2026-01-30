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
pub const XML_VERSION: &str = "1.1";
pub const XML_ENCODING: &str = "UTF-8";
pub const DEFAULT_DATETIME: &str = "1970-01-01T00:00:00Z";
pub const TAG_SCAN_INFO: &str = "ScanInfo";
pub const TAG_GRANDPERSPECTIVE_SCAN_DUMP: &str = "GrandPerspectiveScanDump";
pub const TAG_FOLDER: &str = "Folder";
pub const TAG_FILE: &str = "File";

/// Sanitizes and escapes a string for use in XML attributes and content.
/// This function:
/// 1. Replaces control characters (0x00-0x1F, 0x7F, excluding Tab/LF/CR) with numeric character references
/// 2. Escapes XML special characters (<, >, &, ", ')
/// This ensures the output is valid XML that can be parsed by GrandPerspective.
pub fn sanitize_for_xml(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            // Control characters and DEL (except valid XML whitespace)
            '\x00'..='\x08' | '\x0b'..='\x0c' | '\x0e'..='\x1f' | '\x7f' => {
                result.push_str(&format!("&#x{:X};", c as u32));
            }
            // Valid XML whitespace - pass through
            '\x09' | '\x0a' | '\x0d' => {
                result.push(c);
            }
            // XML special characters
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            // Regular characters
            _ => result.push(c),
        }
    }
    result
}

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
    fn test_sanitize_for_xml() {
        // Test control character 0x0C (^L)
        assert_eq!(sanitize_for_xml("file\x0c.txt"), "file&#xC;.txt");

        // Test DEL character (0x7F)
        assert_eq!(sanitize_for_xml("file\x7f.txt"), "file&#x7F;.txt");

        // Test multiple control characters (0x0B is vertical tab, should be escaped)
        assert_eq!(sanitize_for_xml("a\x0cb\x0bc"), "a&#xC;b&#xB;c");

        // Test that valid XML whitespace is not escaped (Tab, LF, CR are allowed)
        assert_eq!(sanitize_for_xml("a\tb\nc\rd"), "a\tb\nc\rd");

        // Test XML special characters
        assert_eq!(sanitize_for_xml("a&b"), "a&amp;b");
        assert_eq!(sanitize_for_xml("a<b"), "a&lt;b");
        assert_eq!(sanitize_for_xml("a>b"), "a&gt;b");
        assert_eq!(sanitize_for_xml("a\"b"), "a&quot;b");
        assert_eq!(sanitize_for_xml("a'b"), "a&apos;b");

        // Test combined: control character and special character
        assert_eq!(sanitize_for_xml("file\x0c&.txt"), "file&#xC;&amp;.txt");

        // Test normal string
        assert_eq!(sanitize_for_xml("normal.txt"), "normal.txt");

        // Test control character at different positions
        assert_eq!(sanitize_for_xml("\x0cfile"), "&#xC;file");
        assert_eq!(sanitize_for_xml("file\x0c"), "file&#xC;");

        // Test null character
        assert_eq!(sanitize_for_xml("file\x00.txt"), "file&#x0;.txt");
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
        assert!(output.contains(r#"<?xml version="1.1" encoding="UTF-8"?>"#));
        assert!(output.contains(r#"<GrandPerspectiveScanDump"#));
        assert!(output.contains(r#"appVersion="4""#));
        assert!(output.contains(r#"formatVersion="7""#));
    }

    #[test]
    fn test_constants() {
        assert_eq!(GRANDPERSPECTIVE_APP_VERSION, "4");
        assert_eq!(GRANDPERSPECTIVE_FORMAT_VERSION, "7");
        assert_eq!(XML_VERSION, "1.1");
        assert_eq!(XML_ENCODING, "UTF-8");
        assert_eq!(DEFAULT_DATETIME, "1970-01-01T00:00:00Z");
        assert_eq!(TAG_SCAN_INFO, "ScanInfo");
        assert_eq!(TAG_GRANDPERSPECTIVE_SCAN_DUMP, "GrandPerspectiveScanDump");
        assert_eq!(TAG_FOLDER, "Folder");
        assert_eq!(TAG_FILE, "File");
    }
}
