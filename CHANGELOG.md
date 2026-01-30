# Changelog

Notable changes to this project will be documented in this file.

## [0.1.4] - 2026-01-30

Fixed:
- Control characters in filenames and directory names now properly escaped as numeric entities (e.g., `&#xC;`)
- GrandPerspective can now read XML files with control characters in filenames

Changed:
- XML version upgraded from 1.0 to 1.1 for specification compliance with control character numeric entities

## [0.1.3] - 2026-01-20

Changed:
- Root folder name now displays as volume-relative path instead of full filesystem path

## [0.1.2] - 2025-12-18

Changed:
- Execution time now logged via logger (respects `--quiet` flag)

## [0.1.1] - 2025-11-12

Changed:
- Improved cross-platform path handling for output filenames

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
