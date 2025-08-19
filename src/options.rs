use crate::compression::CompressionType;
use clap::ArgMatches;

pub struct Options {
    pub apparent_size: bool,
    pub cross_mount_points: bool,
    pub include_zero_files: bool,
    pub include_empty_folders: bool,
    pub compression_type: CompressionType,
}

impl Options {
    pub fn from_matches(matches: &ArgMatches) -> Self {
        // Determine compression type from flags or output filename
        let compression_type = if matches.get_flag("gzip") {
            CompressionType::Gzip
        } else if matches.get_flag("zstd") {
            CompressionType::Zstd
        } else if let Some(output_file) = matches.get_one::<String>("output") {
            CompressionType::from_extension(output_file)
        } else {
            CompressionType::None
        };

        Options {
            apparent_size: matches.get_flag("apparent-size"),
            cross_mount_points: matches.get_flag("mounts"),
            include_zero_files: matches.get_flag("zero-files"),
            include_empty_folders: matches.get_flag("empty-folders"),
            compression_type,
        }
    }

    /// Get compression type for stdout (only explicit flags, not file extension)
    pub fn compression_type_for_stdout(matches: &ArgMatches) -> CompressionType {
        if matches.get_flag("gzip") {
            CompressionType::Gzip
        } else if matches.get_flag("zstd") {
            CompressionType::Zstd
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
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("zstd"),
            )
            .arg(
                Arg::new("zstd")
                    .short('s')
                    .long("zstd")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("gzip"),
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
    fn test_options_compression_from_extension() {
        let app = create_test_command();

        // Test gzip extension detection
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "file.gz"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Gzip);

        // Test zstd extension detection
        let matches = app
            .clone()
            .try_get_matches_from(vec!["test", "--output", "file.zst"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Zstd);

        // Test explicit flag overrides extension
        let matches = app
            .try_get_matches_from(vec!["test", "--output", "file.txt", "--zstd"])
            .unwrap();
        let options = Options::from_matches(&matches);
        assert_eq!(options.compression_type, CompressionType::Zstd);
    }
}
