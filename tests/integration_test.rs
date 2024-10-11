use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use tempdir::TempDir;

#[test]
fn test_gpscan_output() {
    // Set up a temporary directory structure for testing:
    //
    // gpscan_test
    // ├── file1.txt
    // ├── empty_file.txt
    // ├── subdir
    // │   └── file2.txt
    // └── empty_dir
    //
    let temp_dir = TempDir::new("gpscan_test").expect("Failed to create temp dir");
    let dir_path = temp_dir.path();

    // Create a non-empty file with sample content
    let mut file1 = File::create(dir_path.join("file1.txt")).expect("Failed to create file1");
    writeln!(file1, "This is a test file.").expect("Failed to write to file1");

    // Create an empty file
    File::create(dir_path.join("empty_file.txt")).expect("Failed to create empty_file");

    // Create a subdirectory
    fs::create_dir(dir_path.join("subdir")).expect("Failed to create subdir");

    // Create a non-empty file inside the subdirectory
    let mut file2 =
        File::create(dir_path.join("subdir").join("file2.txt")).expect("Failed to create file2");
    writeln!(file2, "This is another test file.").expect("Failed to write to file2");

    // Create an empty directory
    fs::create_dir(dir_path.join("empty_dir")).expect("Failed to create empty_dir");

    // Run `gpscan` without additional arguments and capture its output
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap());
    let output = cmd.output().expect("Failed to execute gpscan");
    let xml_output = String::from_utf8_lossy(&output.stdout);

    // Check if the XML output contains expected entries for non-empty files and folders
    assert!(
        predicate::str::contains(r#"<File name="file1.txt""#).eval(&xml_output),
        "XML output does not contain file1.txt"
    );
    assert!(
        predicate::str::contains(r#"<Folder name="subdir""#).eval(&xml_output),
        "XML output does not contain subdir"
    );
    assert!(
        predicate::str::contains(r#"<File name="file2.txt""#).eval(&xml_output),
        "XML output does not contain file2.txt"
    );

    // Check if the XML output does NOT contain empty files and empty folders
    assert!(
        !predicate::str::contains(r#"<File name="empty_file.txt""#).eval(&xml_output),
        "XML output contains empty_file.txt"
    );
    assert!(
        !predicate::str::contains(r#"<Folder name="empty_dir""#).eval(&xml_output),
        "XML output contains empty_dir"
    );

    // Check the start and end tags of the XML output
    assert!(
        predicate::str::starts_with(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<GrandPerspectiveScanDump"#
        )
        .eval(&xml_output),
        "XML output does not start with <GrandPerspectiveScanDump>"
    );
    assert!(
        predicate::str::ends_with(r#"</GrandPerspectiveScanDump>"#).eval(&xml_output.trim_end()),
        "XML output does not end with </GrandPerspectiveScanDump>"
    );

    // Test --include-zero-files option: check if empty files are included in the XML output
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("--include-zero-files");
    let output = cmd.output().expect("Failed to execute gpscan");
    let xml_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        predicate::str::contains(r#"<File name="empty_file.txt""#).eval(&xml_output),
        "XML output does not contain empty_file.txt"
    );

    // Test --include-empty-folders option: check if empty folders are included in the XML output
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("--include-empty-folders");
    let output = cmd.output().expect("Failed to execute gpscan");
    let xml_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        predicate::str::contains(r#"<Folder name="empty_dir""#).eval(&xml_output),
        "XML output does not contain empty_dir"
    );
}
