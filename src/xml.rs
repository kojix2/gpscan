/// Escapes special characters for XML output.
pub fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            c if c.is_control() || c == '\u{FFFD}' => format!("&#x{:X};", c as u32),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::xml_escape;

    /// Tests basic escaping of individual special characters
    #[test]
    fn test_xml_escape_basic() {
        assert_eq!(xml_escape("&"), "&amp;");
        assert_eq!(xml_escape("<"), "&lt;");
        assert_eq!(xml_escape(">"), "&gt;");
        assert_eq!(xml_escape("\""), "&quot;");
        assert_eq!(xml_escape("'"), "&apos;");
    }

    /// Tests escaping when mixed with regular text
    #[test]
    fn test_xml_escape_mixed() {
        assert_eq!(
            xml_escape("Hello & Welcome <world>"),
            "Hello &amp; Welcome &lt;world&gt;"
        );
    }

    /// Tests escaping of control characters (ASCII 0x00 to 0x1F)
    #[test]
    fn test_xml_escape_control_chars() {
        assert_eq!(xml_escape("\u{0000}"), "&#x0;"); // Null character
        assert_eq!(xml_escape("\u{0008}"), "&#x8;"); // Backspace
    }

    /// Tests that normal text without special characters is not altered
    #[test]
    fn test_xml_escape_regular_text() {
        assert_eq!(xml_escape("Hello, world!"), "Hello, world!");
        assert_eq!(
            xml_escape("Rust programming language"),
            "Rust programming language"
        );
    }

    /// Tests edge cases such as empty strings and numeric characters
    #[test]
    fn test_xml_escape_edge_cases() {
        assert_eq!(xml_escape(""), ""); // Empty string
        assert_eq!(xml_escape("1234567890"), "1234567890"); // Numbers only
    }
}
