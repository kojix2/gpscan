# gpscan

`gpscan` is a Rust tool that recursively scans a directory and generates an XML file compatible with [GrandPerspective](https://grandperspectiv.sourceforge.net/), a disk usage visualization tool for macOS.

## Features

- Recursively scans directories and files
- Generates XML output compatible with GrandPerspective
- Skips symbolic links to prevent infinite loops
- Handles file permissions and errors gracefully

## Installation

### Download Pre-built Binary

You can download the pre-built binary from the [GitHub Releases](https://github.com/yourusername/gpscan/releases) page.

### Build from Source

Alternatively, build `gpscan` from source:

```sh
git clone https://github.com/yourusername/gpscan.git
cd gpscan
cargo build --release
```

The compiled binary will be located at `target/release/gpscan`.

## Usage

Use `gpscan` to scan a directory on any system (e.g., Linux) and view the results on macOS using GrandPerspective:

```sh
gpscan <directory_path> > scan_result.xml
```

**Example:**

```sh
gpscan /var/log > scan_result.xml
```

Transfer the `scan_result.xml` file to your macOS machine, then open it in GrandPerspective via `File` > `Load Scan Data...`.

This allows you to analyze disk usage on remote servers or systems and visualize it with the macOS GrandPerspective GUI.

## Development

This project was created entirely using ChatGPT. Please be aware that some people believe AI-generated code may not fully comply with the MIT License.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
