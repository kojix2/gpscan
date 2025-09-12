use flate2::write::GzEncoder;
use flate2::Compression as GzipCompression;
use std::io::{self, Write};

/// Enumeration representing compression types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
}

impl CompressionType {
    /// Determine compression type from file extension
    pub fn from_extension(filename: &str) -> Self {
        // Check extensions in order of preference
        const EXTENSIONS: &[(&str, CompressionType)] = &[(".gz", CompressionType::Gzip)];

        for (ext, compression_type) in EXTENSIONS {
            if filename.ends_with(ext) {
                return *compression_type;
            }
        }

        CompressionType::None
    }
}

/// Factory function to create compressed writers
pub fn create_compressed_writer<W: Write + 'static>(
    writer: W,
    compression_type: CompressionType,
) -> io::Result<Box<dyn Write>> {
    match compression_type {
        CompressionType::None => Ok(Box::new(writer)),
        CompressionType::Gzip => {
            let encoder = GzEncoder::new(writer, GzipCompression::default());
            Ok(Box::new(encoder))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_compression_type_from_extension() {
        assert_eq!(
            CompressionType::from_extension("file.txt"),
            CompressionType::None
        );
        assert_eq!(
            CompressionType::from_extension("file.gz"),
            CompressionType::Gzip
        );
        assert_eq!(
            CompressionType::from_extension("file.xml.gz"),
            CompressionType::Gzip
        );
    }

    #[test]
    fn test_create_compressed_writer_none() {
        let buffer = Cursor::new(Vec::new());
        let writer = create_compressed_writer(buffer, CompressionType::None);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_create_compressed_writer_gzip() {
        let buffer = Cursor::new(Vec::new());
        let writer = create_compressed_writer(buffer, CompressionType::Gzip);
        assert!(writer.is_ok());
    }
}
