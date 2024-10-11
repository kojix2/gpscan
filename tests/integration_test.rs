use assert_cmd::Command;
use predicates::prelude::*; // For predicate functions
use std::fs::{self, File};
use std::io::Write;
use tempdir::TempDir;

#[test]
fn test_gpscan_output() {
    // Create a temporary directory
    let temp_dir = TempDir::new("gpscan_test").expect("Failed to create temp dir");
    let dir_path = temp_dir.path();

    // Create sample files and directories
    fs::create_dir(dir_path.join("subdir")).expect("Failed to create subdir");

    let mut file1 = File::create(dir_path.join("file1.txt")).expect("Failed to create file1");
    writeln!(file1, "This is a test file.").expect("Failed to write to file1");

    let mut file2 =
        File::create(dir_path.join("subdir").join("file2.txt")).expect("Failed to create file2");
    writeln!(file2, "This is another test file.").expect("Failed to write to file2");

    // Build the gpscan binary
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");

    // Run the gpscan command
    cmd.arg(dir_path.to_str().unwrap());

    // Assert that the command runs successfully
    cmd.assert().success();

    // Capture the output
    let output = cmd.output().expect("Failed to execute gpscan");
    let xml_output = String::from_utf8_lossy(&output.stdout);

    // Create predicates to check the XML output
    let file1_predicate = predicate::str::contains(r#"<File name="file1.txt""#);
    let subdir_predicate = predicate::str::contains(r#"<Folder name="subdir""#);
    let file2_predicate = predicate::str::contains(r#"<File name="file2.txt""#);
    let start_tag_predicate = predicate::str::starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let end_tag_predicate = predicate::str::ends_with(r#"</GrandPerspectiveScanDump>"#);

    // Check that the XML output contains expected entries
    assert!(
        file1_predicate.eval(&xml_output),
        "XML output does not contain file1.txt"
    );
    assert!(
        subdir_predicate.eval(&xml_output),
        "XML output does not contain subdir"
    );
    assert!(
        file2_predicate.eval(&xml_output),
        "XML output does not contain file2.txt"
    );

    // Check that the XML starts and ends with the correct tags
    assert!(
        start_tag_predicate.eval(&xml_output),
        "XML output does not start with <GrandPerspectiveScanDump>"
    );
    assert!(
        end_tag_predicate.eval(&xml_output.trim_end()),
        "XML output does not end with </GrandPerspectiveScanDump>"
    );
}
