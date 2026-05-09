use std::fs::Metadata;
use std::io;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
use std::path::Path;

pub type FileIdentity = (u64, u64);

pub trait MetadataExtOps {
    fn device_id(&self) -> u64;
    fn inode_number(&self) -> u64;
    fn file_size(&self, apparent: bool) -> u64;
}

#[cfg(not(target_os = "windows"))]
pub fn file_identity(_path: &Path, metadata: &Metadata) -> io::Result<Option<FileIdentity>> {
    let identity = (metadata.device_id(), metadata.inode_number());
    Ok((identity != (0, 0)).then_some(identity))
}

#[cfg(target_os = "windows")]
pub fn file_identity(path: &Path, _metadata: &Metadata) -> io::Result<Option<FileIdentity>> {
    use std::fs::File;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path)?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    let file_index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let identity = (info.dwVolumeSerialNumber as u64, file_index);
    Ok((identity != (0, 0)).then_some(identity))
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
            self.st_size()
        } else {
            self.st_blocks() * 512
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
        // `std::os::windows::fs::MetadataExt::volume_serial_number()` is still unstable.
        // Use `file_identity` when stable file identity is needed.
        0
    }

    fn inode_number(&self) -> u64 {
        // `std::os::windows::fs::MetadataExt::file_index()` is still unstable.
        // Use `file_identity` when stable file identity is needed.
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
