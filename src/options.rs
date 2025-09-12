use crate::compression::CompressionType;
use clap::ArgMatches;

pub struct Options {
    pub apparent_size: bool,
    pub cross_mount_points: bool,
    pub include_zero_files: bool,
    pub include_empty_folders: bool,
    pub compression_type: CompressionType,
    pub output_filename: Option<String>,
}

impl Options {
    pub fn from_matches(matches: &ArgMatches) -> Self {
        let output_file = matches.get_one::<String>("output");
        let no_gzip = matches.get_flag("no-gzip");
        let gzip_flag = matches.get_flag("gzip");

        // Determine compression type and output filename
        let (compression_type, output_filename) = match output_file {
            Some(filename) => {
                // File output: default to gzip unless --no-gzip is specified
                let compression = if no_gzip {
                    CompressionType::None
                } else {
                    CompressionType::Gzip
                };
                let final_filename = Self::process_output_filename(filename);
                (compression, Some(final_filename))
            }
            None => {
                // Stdout: default to no compression unless --gzip is specified
                let compression = if gzip_flag {
                    CompressionType::Gzip
                } else {
                    CompressionType::None
                };
                (compression, None)
            }
        };

        Options {
            apparent_size: matches.get_flag("apparent-size"),
            cross_mount_points: matches.get_flag("mounts"),
            include_zero_files: matches.get_flag("zero-files"),
            include_empty_folders: matches.get_flag("empty-folders"),
            compression_type,
            output_filename,
        }
    }

    /// Process output filename to add .gpscan extension if needed
    fn process_output_filename(filename: &str) -> String {
        if filename.ends_with(".gpscan") {
            filename.to_string()
        } else {
            format!("{}.gpscan", filename)
        }
    }

    /// Get compression type for stdout (only explicit flags, not file extension)
    pub fn compression_type_for_stdout(matches: &ArgMatches) -> CompressionType {
        if matches.get_flag("gzip") {
            CompressionType::Gzip
        } else {
            CompressionType::None
        }
    }

    /// Create Options with default values for testing
    pub fn default() -> Self {
        Options {
            apparent_size: false,
            cross_mount_points: false,
            include_zero_files: false,
            include_empty_folders: false,
            compression_type: CompressionType::None,
            output_filename: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};

    /// Helper function to create a test command with all arguments
    fn create_test_command() -> Command {
        Command::new("test")
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("FILE")
                    .num_args(1),
            )
            .arg(
                Arg::new("apparent-size")
                    .long("apparent-size")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("mounts")
                    .long("mounts")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("zero-files")
                    .long("zero-files")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("empty-folders")
                    .long("empty-folders")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("gzip")
                    .short('z')
                    .long("gzip")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("no-gzip")
                    .long("no-gzip")
                    .action(clap::ArgAction::SetTrue),
            )
    }

    #[test]
    fn test_options_from_matches_default() {
        let app = create_test_command();

        let matches = app.try_get_matches_from(vec!["test"]).unwrap();
        let options = Options::from_matches(&matches);

        assert!(!options.apparent_size);
        assert!(!options.cross_mount_points);
        assert!(!options.include_zero_files);
        assert!(!options.include_empty_folders);
        assert_eq!(options.compression_type, CompressionType::None);
    }

    #[test]
    fn test_options_from_matches_all_flags() {
        let app = create_test_command();

        let matches = app
            .try_get_matches_from(vec![
                "test",
                "--apparent-size",
                "--mounts",
                "--zero-files",
                "--empty-folders",
                "--gzip",
            ])
            .unwrap();
        let options = Options::from_matches(&matches);

        assert!(options.apparent_size);
        assert!(options.cross_mount_points);
        assert!(options.include_zero_files);
        assert!(options.include_empty_folders);
        assert_eq!(options.compression_type, CompressionType::Gzip);
    }

    #[test]
    fn test_options_default() {
        let options = Options::default();

        assert!(!options.apparent_size);
        assert!(!options.cross_mount_points);
        assert!(!options.include_zero_files);
        assert!(!options.include_empty_folders);
        assert_eq!(options.compression_type, CompressionType::None);
    }

    #[test]
    fn test_file_output_default_gzip() {
        let app = create_test_command();

        // Test file output defaults to gzip compression
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "foo"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Gzip);
        assert_eq!(options.output_filename, Some("foo.gpscan".to_string()));
    }

    #[test]
    fn test_file_output_with_gpscan_extension() {
        let app = create_test_command();

        // Test file output with .gpscan extension doesn't add another extension
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "foo.gpscan"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Gzip);
        assert_eq!(options.output_filename, Some("foo.gpscan".to_string()));
    }

    #[test]
    fn test_file_output_with_gz_extension() {
        let app = create_test_command();

        // Test file output with .gz extension gets .gpscan added
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "foo.gz"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Gzip);
        assert_eq!(options.output_filename, Some("foo.gz.gpscan".to_string()));
    }

    #[test]
    fn test_file_output_no_gzip() {
        let app = create_test_command();

        // Test --no-gzip disables compression for file output
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "foo", "--no-gzip"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::None);
        assert_eq!(options.output_filename, Some("foo.gpscan".to_string()));
    }

    #[test]
    fn test_stdout_default_no_compression() {
        let app = create_test_command();

        // Test stdout defaults to no compression
        let matches = app.clone().try_get_matches_from(vec!["test"]).unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::None);
        assert_eq!(options.output_filename, None);
    }

    #[test]
    fn test_stdout_with_gzip() {
        let app = create_test_command();

        // Test --gzip enables compression for stdout
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--gzip"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Gzip);
        assert_eq!(options.output_filename, None);
    }

    #[test]
    fn test_process_output_filename() {
        assert_eq!(Options::process_output_filename("foo"), "foo.gpscan");
        assert_eq!(Options::process_output_filename("foo.gpscan"), "foo.gpscan");
        assert_eq!(Options::process_output_filename("foo.gz"), "foo.gz.gpscan");
        assert_eq!(
            Options::process_output_filename("foo.xml"),
            "foo.xml.gpscan"
        );
    }
}
