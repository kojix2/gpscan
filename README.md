# gpscan

[![Cargo Build & Test](https://github.com/kojix2/gpscan/actions/workflows/ci.yml/badge.svg)](https://github.com/kojix2/gpscan/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gpscan?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fgpscan)](https://crates.io/crates/gpscan)
[![Crates.io](https://img.shields.io/crates/l/gpscan?link=https%3A%2F%2Fgithub.com%2Fgpscan-community%2Fgpscan%2Fblob%2Fmain%2FLICENCE)](LICENSE)

Scan your Linux filesystem and get an XML file compatible with [GrandPerspective](https://grandperspectiv.sourceforge.net/) on macOS to visualize disk usage.

- Recursively scans directories and files
- Generates XML output compatible with GrandPerspective
- Skips symbolic links to prevent infinite loops
- Handles file permissions and errors

## Installation

### Downloading

You can download prebuilt binaries in the [GitHub Releases](https://github.com/kojix2/gpscan/releases).

### Building

```sh
cargo install gpscan
```

## Usage

### Basic usage

```sh
gpscan ./foo > result.gpscan
```

```sh
gpscan / | gzip -c > result.gpscan.gz
```

1. Transfer the `result.gpscan` file to your Mac.
2. Open it in [GrandPerspective](https://grandperspectiv.sourceforge.net/).

## Development

```sh
git clone https://github.com/kojix2/gpscan.git
cd gpscan
cargo build --release
```

## License

[MIT](LICENSE)

This project was created using the full assistance of ChatGPT.
