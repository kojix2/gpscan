use clap::{Arg, ArgMatches, Command};

/// Parses command-line arguments using clap.
pub fn parse_args() -> ArgMatches {
    let bold_underline = "\x1b[1;4m";
    let bold = "\x1b[1m";
    let reset = "\x1b[0m";

    Command::new("gpscan")
        .version(clap::crate_version!())
        .about(&format!(
            "\n\n{}Program:{} {}gpscan{} (GrandPerspective XML Scan Dump)\n\
            Version: {}\n\
            Source:  https://github.com/kojix2/gpscan",
            bold_underline,
            reset,
            bold,
            reset,
            clap::crate_version!()
        ))
        .arg(
            Arg::new("directory")
                .help("The directory to scan (required)")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("mounts")
                .short('m')
                .long("mounts")
                .help("Cross filesystem boundaries during scan [false]")
                .num_args(0),
        )
        .arg(
            Arg::new("include-zero-files")
                .short('z')
                .long("include-zero-files")
                .help("Include zero-byte files in the scan output [false]")
                .num_args(0),
        )
        .arg(
            Arg::new("include-empty-folders")
                .short('e')
                .long("include-empty-folders")
                .help("Include empty folders in the scan output [false]")
                .num_args(0),
        )
        .arg_required_else_help(true)
        .get_matches()
}
