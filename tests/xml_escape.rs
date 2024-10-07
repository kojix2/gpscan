use gpscan::xml_escape;

#[test]
fn test_xml_escape() {
    // Test if special characters are correctly escaped
    let input = "&<>'\"";
    let expected = "&amp;&lt;&gt;&apos;&quot;";
    assert_eq!(xml_escape(input), expected);

    // Test if a string without special characters is returned unchanged
    let input_no_escape = "Hello, World!";
    let expected_no_escape = "Hello, World!";
    assert_eq!(xml_escape(input_no_escape), expected_no_escape);
}
