use std::fs::Metadata;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

pub trait MetadataExtOps {
    fn device_id(&self) -> u64;
    fn inode_number(&self) -> u64;
    fn file_size(&self, apparent: bool) -> u64;
}

#[cfg(target_os = "linux")]
impl MetadataExtOps for Metadata {
    fn device_id(&self) -> u64 {
        self.st_dev()
    }

    fn inode_number(&self) -> u64 {
        self.st_ino()
    }

    fn file_size(&self, apparent: bool) -> u64 {
        if apparent {
            self.st_size() as u64
        } else {
            self.st_blocks() as u64 * 512
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
impl MetadataExtOps for Metadata {
    fn device_id(&self) -> u64 {
        self.dev()
    }

    fn inode_number(&self) -> u64 {
        self.ino()
    }

    fn file_size(&self, apparent: bool) -> u64 {
        if apparent {
            self.size()
        } else {
            self.blocks() * 512
        }
    }
}

#[cfg(target_os = "windows")]
impl MetadataExtOps for Metadata {
    fn device_id(&self) -> u64 {
        // NOTE: Previously used `volume_serial_number()` from unstable `windows_by_handle` feature.
        // To maintain compatibility with Rust stable, we return 0 as a placeholder.
        // This method is currently unused in the codebase (dead code).
        // If `windows_by_handle` is stabilized in the future, we can restore the actual implementation:
        //   self.volume_serial_number().unwrap_or(0) as u64
        0
    }

    fn inode_number(&self) -> u64 {
        // NOTE: Previously used `file_index()` from unstable `windows_by_handle` feature.
        // To maintain compatibility with Rust stable, we return 0 as a placeholder.
        // This method is currently unused in the codebase (dead code).
        // If `windows_by_handle` is stabilized in the future, we can restore the actual implementation:
        //   self.file_index().unwrap_or(0)
        0
    }

    fn file_size(&self, apparent: bool) -> u64 {
        if apparent {
            self.len()
        } else {
            // Calculate physical size by rounding up to cluster size
            // Default cluster size is 4KB for most modern Windows systems
            let cluster_size = 4096u64;
            let file_size = self.len();

            // Round up to the nearest cluster boundary
            if file_size == 0 {
                0
            } else {
                ((file_size + cluster_size - 1) / cluster_size) * cluster_size
            }
        }
    }
}
