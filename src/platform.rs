use std::fs::Metadata;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

#[cfg(target_os = "windows")]
use log::warn;

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
            self.size() as u64
        } else {
            self.blocks() as u64 * 512
        }
    }
}

#[cfg(target_os = "windows")]
impl MetadataExtOps for Metadata {
    fn device_id(&self) -> u64 {
        self.volume_serial_number().unwrap_or(0) as u64
    }

    fn inode_number(&self) -> u64 {
        // Windows does not have inode, so use file index
        self.file_index().unwrap_or(0)
    }

    fn file_size(&self, apparent: bool) -> u64 {
        if apparent {
            self.len()
        } else {
            // Physical size is unavailable on Windows, so output a warning
            warn!("Using logical size as physical size is unavailable on Windows.");
            self.len()
        }
    }
}
