use assert_cmd::Command;
use flate2::read::GzDecoder;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink;
#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;
use std::path::Path;
use tempdir::TempDir;

/// Helper function to create a test directory structure
fn create_test_directory(name: &str) -> TempDir {
    let temp_dir = TempDir::new(name).expect("Failed to create temp dir");
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

    // Create symbolic links
    symlink(
        dir_path.join("file1.txt"),
        dir_path.join("symlink_to_file1"),
    )
    .expect("Failed to create symlink to file1");
    symlink(dir_path.join("subdir"), dir_path.join("symlink_to_subdir"))
        .expect("Failed to create symlink to subdir");

    // Create a hard link inside the subdirectory
    std::fs::hard_link(
        dir_path.join("subdir").join("file2.txt"),
        dir_path.join("subdir").join("hardlink_to_file2"),
    )
    .expect("Failed to create hard link to file2");

    temp_dir
}

/// Helper function to create a simple test directory for compression tests
fn create_simple_test_directory(name: &str, content1: &str, content2: &str) -> TempDir {
    let temp_dir = TempDir::new(name).expect("Failed to create temp dir");
    let dir_path = temp_dir.path();

    // Create a sample file
    let mut file1 = File::create(dir_path.join("file1.txt")).expect("Failed to create file1");
    writeln!(file1, "{}", content1).expect("Failed to write to file1");

    // Create a subdirectory with a file
    fs::create_dir(dir_path.join("subdir")).expect("Failed to create subdir");
    let mut file2 =
        File::create(dir_path.join("subdir").join("file2.txt")).expect("Failed to create file2");
    writeln!(file2, "{}", content2).expect("Failed to write to file2");

    temp_dir
}

/// Helper function to run gpscan command with arguments
fn run_gpscan(dir_path: &Path, args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap());
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("Failed to execute gpscan");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper function to verify XML structure
fn assert_xml_structure(xml_content: &str) {
    assert!(
        predicate::str::starts_with(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<GrandPerspectiveScanDump"#
        )
        .eval(xml_content),
        "XML does not start with correct declaration"
    );
    assert!(
        predicate::str::ends_with("</GrandPerspectiveScanDump>").eval(xml_content.trim_end()),
        "XML does not end with </GrandPerspectiveScanDump>"
    );
}

/// Helper function to verify file presence in XML
fn assert_file_in_xml(xml_content: &str, filename: &str, should_exist: bool) {
    let pattern = format!(r#"<File name="{}""#, filename);
    let contains = predicate::str::contains(&pattern).eval(xml_content);
    if should_exist {
        assert!(contains, "XML does not contain {}", filename);
    } else {
        assert!(!contains, "XML contains {}", filename);
    }
}

/// Helper function to verify folder presence in XML
fn assert_folder_in_xml(xml_content: &str, foldername: &str, should_exist: bool) {
    let pattern = format!(r#"<Folder name="{}""#, foldername);
    let contains = predicate::str::contains(&pattern).eval(xml_content);
    if should_exist {
        assert!(contains, "XML does not contain folder {}", foldername);
    } else {
        assert!(!contains, "XML contains folder {}", foldername);
    }
}

/// Helper function to decompress gzip file and return content
fn decompress_gzip_file(file_path: &Path) -> String {
    let compressed_file = File::open(file_path).expect("Failed to open compressed file");
    let mut decoder = GzDecoder::new(compressed_file);
    let mut decompressed_content = String::new();
    decoder
        .read_to_string(&mut decompressed_content)
        .expect("Failed to decompress gzip file");
    decompressed_content
}

#[test]
fn test_gpscan_output() {
    // Set up a temporary directory structure for testing:
    //
    // gpscan_test
    // ├── file1.txt
    // ├── empty_file.txt
    // ├── subdir
    // │   ├── file2.txt
    // │   └── hardlink_to_file2 -> file2.txt
    // ├── empty_dir
    // ├── symlink_to_file1 -> file1.txt
    // └── symlink_to_subdir -> subdir
    //
    let temp_dir = create_test_directory("gpscan_test");
    let dir_path = temp_dir.path();

    // Run `gpscan` and capture its output
    let xml_output = run_gpscan(dir_path, &[]);

    // Debug print the XML output
    println!("{}", xml_output);

    // Check if the XML output contains expected entries for non-empty files and folders
    assert_file_in_xml(&xml_output, "file1.txt", true);
    assert_folder_in_xml(&xml_output, "subdir", true);
    assert_file_in_xml(&xml_output, "file2.txt", true);

    // Check if the XML output does NOT contain empty files, empty folders, or symbolic links
    assert_file_in_xml(&xml_output, "empty_file.txt", false);
    assert_folder_in_xml(&xml_output, "empty_dir", false);
    assert_file_in_xml(&xml_output, "symlink_to_file1", false);
    assert_folder_in_xml(&xml_output, "symlink_to_subdir", false);
    assert_file_in_xml(&xml_output, "hardlink_to_file2", false);

    // Test XML structure
    assert_xml_structure(&xml_output);
    assert!(
        xml_output.contains("fileSizeMeasure=\"physical\""),
        "Expected physical measure by default"
    );

    // Test --zero-files option
    let xml_output_zero = run_gpscan(dir_path, &["--zero-files"]);
    assert_file_in_xml(&xml_output_zero, "empty_file.txt", true);
    assert!(xml_output_zero.contains("fileSizeMeasure=\"physical\""));

    // Test --empty-folders option
    let xml_output_empty = run_gpscan(dir_path, &["--empty-folders"]);
    assert_folder_in_xml(&xml_output_empty, "empty_dir", true);
    assert!(xml_output_empty.contains("fileSizeMeasure=\"physical\""));

    // Test apparent size (logical)
    let xml_output_logical = run_gpscan(dir_path, &["--apparent-size"]);
    assert!(
        xml_output_logical.contains("fileSizeMeasure=\"logical\""),
        "Expected logical measure when --apparent-size is set"
    );
}

#[test]
fn test_gpscan_with_output_file() {
    let temp_dir = create_simple_test_directory("gpscan_test_output", "Content for file1", "");
    let dir_path = temp_dir.path();

    // Specify an output file
    let output_file_path = dir_path.join("output.xml");

    // Run `gpscan` with an output file specified
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg(output_file_path.to_str().unwrap());
    cmd.assert().success();

    // Read and verify the output from the specified file
    let output_xml = fs::read_to_string(output_file_path).expect("Failed to read output file");
    assert_file_in_xml(&output_xml, "file1.txt", true);
    assert_xml_structure(&output_xml);
}

#[test]
fn test_gpscan_invalid_output_path() {
    let temp_dir = create_simple_test_directory("gpscan_invalid_output", "test content", "");
    let dir_path = temp_dir.path();

    // Specify an invalid output file path
    let invalid_output_path = dir_path.join("nonexistent_directory/output.xml");

    // Run `gpscan` with an invalid output file path
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg(invalid_output_path.to_str().unwrap());

    // Adjust expectation to match the os error message structure
    #[cfg(target_os = "windows")]
    let expected_error = "The system cannot find the path specified";
    #[cfg(not(target_os = "windows"))]
    let expected_error = "No such file or directory";

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(expected_error));
}

#[test]
fn test_gpscan_with_gzip_compression() {
    let temp_dir = create_simple_test_directory(
        "gpscan_gzip_test",
        "Content for gzip compression test",
        "Another file for gzip test",
    );
    let dir_path = temp_dir.path();

    // Test gzip compression with long flag
    let output_file_path = dir_path.join("output.xml.gz");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg(output_file_path.to_str().unwrap())
        .arg("--gzip");
    cmd.assert().success();

    // Verify the compressed file exists and decompress it
    assert!(
        output_file_path.exists(),
        "Gzip compressed output file was not created"
    );
    let decompressed_content = decompress_gzip_file(&output_file_path);

    // Verify the decompressed XML content
    assert_file_in_xml(&decompressed_content, "file1.txt", true);
    assert_folder_in_xml(&decompressed_content, "subdir", true);
    assert_file_in_xml(&decompressed_content, "file2.txt", true);
    assert_xml_structure(&decompressed_content);

    // Test gzip compression with short flag using a separate directory
    let temp_dir2 = create_simple_test_directory(
        "gpscan_gzip_test2",
        "Content for gzip compression test",
        "Another file for gzip test",
    );
    let dir_path2 = temp_dir2.path();

    let output_file_path2 = dir_path2.join("output2.xml.gz");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path2.to_str().unwrap())
        .arg("-o")
        .arg(output_file_path2.to_str().unwrap())
        .arg("-z");
    cmd.assert().success();

    // Verify the second compressed file exists and is valid
    assert!(
        output_file_path2.exists(),
        "Second gzip compressed output file was not created"
    );
    let decompressed_content2 = decompress_gzip_file(&output_file_path2);

    // Verify both outputs contain the same file structure
    assert_file_in_xml(&decompressed_content2, "file1.txt", true);
    assert_folder_in_xml(&decompressed_content2, "subdir", true);
    assert_file_in_xml(&decompressed_content2, "file2.txt", true);
    assert_xml_structure(&decompressed_content2);
}

#[test]
fn test_gpscan_with_auto_gzip_detection() {
    let temp_dir = create_simple_test_directory(
        "gpscan_auto_gzip_test",
        "Content for auto gzip detection test",
        "",
    );
    let dir_path = temp_dir.path();

    // Test automatic gzip compression based on .gz extension
    let output_file_path = dir_path.join("auto_output.xml.gz");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg(output_file_path.to_str().unwrap());
    cmd.assert().success();

    // Verify the compressed file exists and decompress it
    assert!(
        output_file_path.exists(),
        "Auto-detected gzip compressed output file was not created"
    );
    let decompressed_content = decompress_gzip_file(&output_file_path);

    // Verify the decompressed XML content
    assert_file_in_xml(&decompressed_content, "file1.txt", true);
    assert_xml_structure(&decompressed_content);
}
