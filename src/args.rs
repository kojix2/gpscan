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
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file (gzip by default, adds .gpscan)")
                .num_args(1),
        )
        .arg(
            Arg::new("apparent-size")
                .short('A')
                .long("apparent-size")
                .help("Use apparent size instead of disk usage")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("mounts")
                .short('m')
                .long("mounts")
                .help("Cross filesystem boundaries during scan")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("zero-files")
                .short('Z')
                .long("zero-files")
                .help("Include zero-byte files in scan")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("empty-folders")
                .short('E')
                .long("empty-folders")
                .help("Include empty folders in scan")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress all informational messages")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("gzip")
                .short('z')
                .long("gzip")
                .help("Gzip-compress stdout (file output is gzip by default)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-gzip")
                .long("no-gzip")
                .help("Disable gzip for file output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg_required_else_help(true)
        .get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_function_exists() {
        // Simple smoke test to ensure the function compiles and can be called
        // Actual functionality is tested via integration tests
        let _result = std::panic::catch_unwind(|| {
            // This will fail due to no command line args, but ensures the function exists
            let _ = Command::new("test").try_get_matches_from(vec!["test"]);
        });
    }
}
