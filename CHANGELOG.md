# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2025-09-12

Highlights:
- File output (-o) now defaults to gzip compression and auto-appends the `.gpscan` suffix
- New options: `--no-gzip`, `--compression-level <0-9>`, `--force`
- Safer filename processing and overwrite behavior with TTY prompt
- Help and README updated; tests expanded (including Windows-style paths)

Added:
- `--compression-level <0-9>` to control gzip level (default: 6)
- `--force` to overwrite existing output files without prompt
- TTY overwrite confirmation prompt; non-interactive mode requires `--force`

Changed:
- With `-o/--output`, file output is gzip-compressed by default
- `.gpscan` is automatically appended to output filenames (e.g., `result` -> `result.gpscan`)
- CLI help text simplified and clarified

Fixed/Improved:
- Robust output filename handling (trailing dots, directory-like paths, only filename part modified)
- Integration tests for stdout gzip, default gzip for file output, and overwrite behavior
- Removed obsolete compression extension auto-detection logic

Notes:
- `--compression-level` applies when gzip is enabled. If `--no-gzip` is specified, the level is ignored.

[0.1.0]: https://github.com/kojix2/gpscan/releases/tag/v0.1.0
