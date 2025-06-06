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
                .help("Output file (default: stdout)")
                .num_args(1),
        )
        .arg(
            Arg::new("apparent-size")
                .short('A')
                .long("apparent-size")
                .help("Use apparent size instead of disk usage [false]")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("mounts")
                .short('m')
                .long("mounts")
                .help("Cross filesystem boundaries during scan [false]")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("include-zero-files")
                .short('z')
                .long("include-zero-files")
                .help("Include zero-byte files in scan [false]")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("include-empty-folders")
                .short('e')
                .long("include-empty-folders")
                .help("Include empty folders in scan [false]")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress all informational messages [false]")
                .action(clap::ArgAction::SetTrue),
        )
        .arg_required_else_help(true)
        .get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_with_directory() {
        // We can't easily test the actual parse_args function since it calls get_matches()
        // which expects real command line arguments. Instead, we test the command structure.
        let bold_underline = "\x1b[1;4m";
        let bold = "\x1b[1m";
        let reset = "\x1b[0m";

        let app = Command::new("gpscan")
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
                    .help("Output file (default: stdout)")
                    .num_args(1),
            )
            .arg(
                Arg::new("apparent-size")
                    .short('A')
                    .long("apparent-size")
                    .help("Use apparent size instead of disk usage [false]")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("mounts")
                    .short('m')
                    .long("mounts")
                    .help("Cross filesystem boundaries during scan [false]")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("include-zero-files")
                    .short('z')
                    .long("include-zero-files")
                    .help("Include zero-byte files in scan [false]")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("include-empty-folders")
                    .short('e')
                    .long("include-empty-folders")
                    .help("Include empty folders in scan [false]")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("quiet")
                    .short('q')
                    .long("quiet")
                    .help("Suppress all informational messages [false]")
                    .action(clap::ArgAction::SetTrue),
            );

        // Test with minimal required arguments
        let matches = app
            .clone()
            .try_get_matches_from(vec!["gpscan", "/test/path"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("directory").unwrap(),
            "/test/path"
        );
        assert!(!matches.get_flag("apparent-size"));
        assert!(!matches.get_flag("mounts"));
        assert!(!matches.get_flag("include-zero-files"));
        assert!(!matches.get_flag("include-empty-folders"));
        assert!(!matches.get_flag("quiet"));

        // Test with all flags
        let matches = app
            .try_get_matches_from(vec![
                "gpscan",
                "/test/path",
                "--apparent-size",
                "--mounts",
                "--include-zero-files",
                "--include-empty-folders",
                "--quiet",
                "--output",
                "output.xml",
            ])
            .unwrap();

        assert_eq!(
            matches.get_one::<String>("directory").unwrap(),
            "/test/path"
        );
        assert_eq!(matches.get_one::<String>("output").unwrap(), "output.xml");
        assert!(matches.get_flag("apparent-size"));
        assert!(matches.get_flag("mounts"));
        assert!(matches.get_flag("include-zero-files"));
        assert!(matches.get_flag("include-empty-folders"));
        assert!(matches.get_flag("quiet"));
    }

    #[test]
    fn test_parse_args_missing_directory() {
        let app = Command::new("gpscan")
            .arg(
                Arg::new("directory")
                    .help("The directory to scan (required)")
                    .index(1)
                    .required(true),
            )
            .arg_required_else_help(true);

        // Should fail when no directory is provided
        let result = app.try_get_matches_from(vec!["gpscan"]);
        assert!(result.is_err());
    }
}
