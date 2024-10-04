# gpscan

gpscan is a Rust tool that recursively scans a directory and generates an XML file compatible with GrandPerspective, a disk usage visualization tool for macOS.

## Features

- Recursively scans directories and files
- Outputs XML in a format compatible with GrandPerspective
- Handles symbolic links and access permissions
- Provides detailed error messages
- Cross-platform support

## Usage

```sh
cargo run -- <directory_path>
```

