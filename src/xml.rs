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
