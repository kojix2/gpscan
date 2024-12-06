# gpscan

<a href="https://grandperspectiv.sourceforge.net/"><img src="https://grandperspectiv.sourceforge.net/Images/GrandPerspectiveLogoWithShadow.png" width="120" height="120" align="right"></a>

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

### Homebrew

[homebrew tap](https://github.com/kojix2/homebrew-brew/)

```
brew install kojix2/brew/gpscan
```

### Building

```sh
cargo install gpscan
```

## Usage

### Basic usage

```
gpscan [OPTIONS] <directory>
```

```sh
gpscan ./foo > result.gpscan
```

```sh
gpscan / | gzip -c > result.gpscan.gz
```

1. Transfer the `result.gpscan` file to your Mac.
2. Open it in [GrandPerspective](https://grandperspectiv.sourceforge.net/).

### Options

```
  -o, --output <FILE>          Output file (default: stdout)
  -A, --apparent-size          Use apparent size instead of disk usage [false]
  -m, --mounts                 Cross filesystem boundaries during scan [false]
  -z, --include-zero-files     Include zero-byte files in scan [false]
  -e, --include-empty-folders  Include empty folders in scan [false]
  -q, --quiet                  Suppress all informational messages [false]
  -h, --help                   Print help
  -V, --version                Print version
```

## Development

```sh
git clone https://github.com/kojix2/gpscan.git
cd gpscan
cargo build --release
```

## License

[MIT](LICENSE)

This project was created using the full assistance of ChatGPT.
