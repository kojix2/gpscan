use clap::ArgMatches;

pub struct Options {
    pub apparent_size: bool,
    pub cross_mount_points: bool,
    pub include_zero_files: bool,
    pub include_empty_folders: bool,
}

impl Options {
    pub fn from_matches(matches: &ArgMatches) -> Self {
        Options {
            apparent_size: matches.get_flag("apparent-size"),
            cross_mount_points: matches.get_flag("mounts"),
            include_zero_files: matches.get_flag("include-zero-files"),
            include_empty_folders: matches.get_flag("include-empty-folders"),
        }
    }

    /// Create Options with default values for testing
    pub fn default() -> Self {
        Options {
            apparent_size: false,
            cross_mount_points: false,
            include_zero_files: false,
            include_empty_folders: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};

    #[test]
    fn test_options_from_matches_default() {
        let app = Command::new("test")
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
                Arg::new("include-zero-files")
                    .long("include-zero-files")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("include-empty-folders")
                    .long("include-empty-folders")
                    .action(clap::ArgAction::SetTrue),
            );

        let matches = app.try_get_matches_from(vec!["test"]).unwrap();
        let options = Options::from_matches(&matches);

        assert!(!options.apparent_size);
        assert!(!options.cross_mount_points);
        assert!(!options.include_zero_files);
        assert!(!options.include_empty_folders);
    }

    #[test]
    fn test_options_from_matches_all_flags() {
        let app = Command::new("test")
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
                Arg::new("include-zero-files")
                    .long("include-zero-files")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("include-empty-folders")
                    .long("include-empty-folders")
                    .action(clap::ArgAction::SetTrue),
            );

        let matches = app
            .try_get_matches_from(vec![
                "test",
                "--apparent-size",
                "--mounts",
                "--include-zero-files",
                "--include-empty-folders",
            ])
            .unwrap();
        let options = Options::from_matches(&matches);

        assert!(options.apparent_size);
        assert!(options.cross_mount_points);
        assert!(options.include_zero_files);
        assert!(options.include_empty_folders);
    }

    #[test]
    fn test_options_default() {
        let options = Options::default();

        assert!(!options.apparent_size);
        assert!(!options.cross_mount_points);
        assert!(!options.include_zero_files);
        assert!(!options.include_empty_folders);
    }
}
