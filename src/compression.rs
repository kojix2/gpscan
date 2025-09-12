use flate2::write::GzEncoder;
use flate2::Compression as GzipCompression;
use std::io::{self, Write};

/// Enumeration representing compression types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
}

impl CompressionType {}

/// Factory function to create compressed writers
pub fn create_compressed_writer<W: Write + 'static>(
    writer: W,
    compression_type: CompressionType,
) -> io::Result<Box<dyn Write>> {
    // Backward-compat: default level 6
    create_compressed_writer_with_level(writer, compression_type, 6)
}

/// Factory function to create compressed writers with level
pub fn create_compressed_writer_with_level<W: Write + 'static>(
    writer: W,
    compression_type: CompressionType,
    level: u8,
) -> io::Result<Box<dyn Write>> {
    match compression_type {
        CompressionType::None => Ok(Box::new(writer)),
        CompressionType::Gzip => {
            let lvl = if level > 9 { 9 } else { level };
            let encoder = GzEncoder::new(writer, GzipCompression::new(lvl as u32));
            Ok(Box::new(encoder))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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

    #[test]
    fn test_create_compressed_writer_with_level_bounds() {
        let buffer = Cursor::new(Vec::new());
        let w9 = create_compressed_writer_with_level(buffer, CompressionType::Gzip, 9);
        assert!(w9.is_ok());

        let buffer2 = Cursor::new(Vec::new());
        // >9 should be clamped to 9
        let w_over = create_compressed_writer_with_level(buffer2, CompressionType::Gzip, 15);
        assert!(w_over.is_ok());
    }
}
