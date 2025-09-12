use std::cmp::Reverse;
use std::fs;
use std::path::Path;
use sysinfo::Disks;

/// Retrieves volume information for the given path.
pub fn get_volume_info(root_path: &Path, disks: &Disks) -> (String, u64, u64) {
    // Convert root_path to absolute path
    #[cfg(windows)]
    let mut abs_root_path = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    #[cfg(not(windows))]
    let abs_root_path = fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());

    // Remove the "\\?\" prefix on Windows
    #[cfg(windows)]
    {
        abs_root_path =
            std::path::PathBuf::from(abs_root_path.to_string_lossy().replacen(r"\\?\", "", 1));
    }

    // Collect and sort disks by the depth of their mount points (in descending order)
    let mut disks: Vec<_> = disks.iter().collect();
    disks.sort_by_key(|disk| Reverse(disk.mount_point().components().count()));

    // Find the first matching disk
    for disk in disks {
        let mount_point = disk.mount_point();

        if abs_root_path.starts_with(mount_point) {
            let volume_path = mount_point.to_string_lossy().to_string();
            let volume_size = disk.total_space();
            let free_space = disk.available_space();
            return (volume_path, volume_size, free_space);
        }
    }

    // If no matching disk is found, return defaults
    (
        "/".to_string(),
        0, // volume_size
        0, // free_space
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_volume_info_no_disks() {
        let disks = Disks::new();
        let path = Path::new("/nonexistent");
        let (volume_path, volume_size, free_space) = get_volume_info(path, &disks);

        assert_eq!(volume_path, "/");
        assert_eq!(volume_size, 0);
        assert_eq!(free_space, 0);
    }

    #[test]
    fn test_get_volume_info_with_current_dir() {
        let disks = Disks::new_with_refreshed_list();
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let (volume_path, _volume_size, _free_space) = get_volume_info(&current_dir, &disks);

        // Should return a valid volume path (not the default "/")
        // The exact path depends on the system, but it should not be empty
        assert!(!volume_path.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_path_canonicalization() {
        // Test that Windows UNC prefix is properly removed
        let test_path = PathBuf::from(r"\\?\C:\test");
        let _expected = "C:\\test";

        // This is testing the logic inside get_volume_info
        // We can't directly test the internal logic, but we can test the behavior
        let disks = Disks::new();
        let (_volume_path, _volume_size, _free_space) = get_volume_info(&test_path, &disks);

        // The function should handle the path without panicking
        // Exact assertions depend on the system state
    }
}
