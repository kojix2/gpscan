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

    // Specify an output file - should automatically get .gpscan extension and be gzip compressed
    let expected_output_file_path = dir_path.join("output.xml.gpscan");

    // Run `gpscan` with an output file specified
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg("output.xml");
    cmd.current_dir(dir_path);
    cmd.assert().success();

    // The file should be gzip compressed with .gpscan extension
    assert!(
        expected_output_file_path.exists(),
        "Expected output file with .gpscan extension was not created"
    );

    // Decompress and verify the output
    let decompressed_content = decompress_gzip_file(&expected_output_file_path);
    assert_file_in_xml(&decompressed_content, "file1.txt", true);
    assert_xml_structure(&decompressed_content);
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

    // Test file output - should be gzip compressed by default with .gpscan extension
    // Input: "output.xml.gz" -> Output: "output.xml.gz.gpscan" (gzip compressed)
    let expected_output_file_path = dir_path.join("output.xml.gz.gpscan");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg("output.xml.gz");
    cmd.current_dir(dir_path);
    cmd.assert().success();

    // Verify the compressed file exists and decompress it
    assert!(
        expected_output_file_path.exists(),
        "Gzip compressed output file was not created"
    );
    let decompressed_content = decompress_gzip_file(&expected_output_file_path);

    // Verify the decompressed XML content
    assert_file_in_xml(&decompressed_content, "file1.txt", true);
    assert_folder_in_xml(&decompressed_content, "subdir", true);
    assert_file_in_xml(&decompressed_content, "file2.txt", true);
    assert_xml_structure(&decompressed_content);

    // Test --no-gzip flag - should create uncompressed .gpscan file
    let temp_dir2 = create_simple_test_directory(
        "gpscan_no_gzip_test",
        "Content for no gzip test",
        "Another file for no gzip test",
    );
    let dir_path2 = temp_dir2.path();

    let expected_output_file_path2 = dir_path2.join("output2.gpscan");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path2.to_str().unwrap())
        .arg("-o")
        .arg("output2")
        .arg("--no-gzip");
    cmd.current_dir(dir_path2);
    cmd.assert().success();

    // Verify the uncompressed file exists and read it directly
    assert!(
        expected_output_file_path2.exists(),
        "Uncompressed output file was not created"
    );
    let content =
        fs::read_to_string(&expected_output_file_path2).expect("Failed to read uncompressed file");

    // Verify the XML content
    assert_file_in_xml(&content, "file1.txt", true);
    assert_folder_in_xml(&content, "subdir", true);
    assert_file_in_xml(&content, "file2.txt", true);
    assert_xml_structure(&content);
}

#[test]
fn test_gpscan_with_auto_gzip_detection() {
    let temp_dir = create_simple_test_directory(
        "gpscan_auto_gzip_test",
        "Content for auto gzip detection test",
        "",
    );
    let dir_path = temp_dir.path();

    // Test file output with .gpscan extension - should be gzip compressed by default
    let expected_output_file_path = dir_path.join("auto_output.gpscan");
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap())
        .arg("-o")
        .arg("auto_output.gpscan");
    cmd.current_dir(dir_path);
    cmd.assert().success();

    // Verify the compressed file exists and decompress it
    assert!(
        expected_output_file_path.exists(),
        "Auto-detected gzip compressed output file was not created"
    );
    let decompressed_content = decompress_gzip_file(&expected_output_file_path);

    // Verify the decompressed XML content
    assert_file_in_xml(&decompressed_content, "file1.txt", true);
    assert_xml_structure(&decompressed_content);
}

#[test]
fn test_gpscan_stdout_compression() {
    let temp_dir =
        create_simple_test_directory("gpscan_stdout_test", "Content for stdout test", "");
    let dir_path = temp_dir.path();

    // Test stdout without --gzip flag - should output plain text
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap());
    let output = cmd.output().expect("Failed to execute gpscan");
    let stdout_content = String::from_utf8_lossy(&output.stdout);

    // Should be plain XML
    assert_xml_structure(&stdout_content);
    assert_file_in_xml(&stdout_content, "file1.txt", true);

    // Test stdout with --gzip flag - should output compressed data
    let mut cmd = Command::cargo_bin("gpscan").expect("Failed to build gpscan");
    cmd.arg(dir_path.to_str().unwrap()).arg("--gzip");
    let output = cmd.output().expect("Failed to execute gpscan");

    // The output should be gzip compressed (binary data)
    // We can verify this by checking that it's not valid UTF-8 plain text XML
    let stdout_bytes = &output.stdout;
    assert!(stdout_bytes.len() > 0, "No output received");

    // Gzip files start with magic bytes 0x1f, 0x8b
    assert_eq!(stdout_bytes[0], 0x1f, "First byte should be 0x1f for gzip");
    assert_eq!(stdout_bytes[1], 0x8b, "Second byte should be 0x8b for gzip");
}
