//! High-performance xxHash3 checksum implementation.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

/// Incremental streaming xxHash3 hasher.
#[derive(Default, Clone)]
pub struct FastHasher {
    state: Xxh3,
}

impl fmt::Debug for FastHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FastHasher")
            .field("current_digest", &self.state.digest())
            .finish()
    }
}

impl FastHasher {
    /// Creates a new instance of the xxHash3 state.
    #[must_use]
    pub fn new() -> Self {
        Self { state: Xxh3::new() }
    }

    /// Feeds a chunk of bytes into the hash calculation.
    pub fn update(&mut self, chunk: &[u8]) {
        self.state.update(chunk);
    }

    /// Computes the 64-bit digest and returns it formatted as an uppercase hexadecimal string.
    #[must_use]
    pub fn finalize_hex(&self) -> String {
        let hash = self.state.digest();
        format!("{hash:016X}")
    }

    /// Computes the formatted short hash (similar to TeraCopy's format `XXXX:YYYY`).
    #[must_use]
    pub fn finalize_short_hex(&self) -> String {
        let hash = self.state.digest();
        let high = (hash >> 32) as u32;
        let low = hash as u32;
        format!("{high:04X}:{low:04X}")
    }

    /// Computes the 64-bit digest as a raw integer.
    #[must_use]
    pub fn digest(&self) -> u64 {
        self.state.digest()
    }
}

/// Computes the xxHash3 64-bit checksum of a file directly from disk.
/// Uses an 8 MiB buffer for maximal I/O throughput.
///
/// # Errors
/// Returns an `io::Error` if the file cannot be opened or read.
pub fn compute_file_xxhash(
    path: &Path,
    buffer_size: usize,
    buffer: Option<&mut Vec<u8>>,
) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = FastHasher::new();
    let size = buffer_size.max(64 * 1024);

    let mut local_buf;
    let buf_ref = match buffer {
        Some(b) => {
            if b.len() < size {
                b.resize(size, 0);
            }
            b
        }
        None => {
            local_buf = vec![0u8; size];
            &mut local_buf
        }
    };

    loop {
        let bytes_read = file.read(buf_ref)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buf_ref[..bytes_read]);
    }

    Ok(hasher.finalize_short_hex())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xxhash_basic() {
        let mut hasher = FastHasher::new();
        hasher.update(b"Hello World");
        let hash = hasher.finalize_hex();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_xxhash_short_format() {
        let mut hasher = FastHasher::new();
        hasher.update(b"TeraCopy test string");
        let short_hash = hasher.finalize_short_hex();
        assert!(short_hash.contains(':'));
    }
}
