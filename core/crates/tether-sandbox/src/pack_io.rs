//! Streaming pack file hashing.

use std::fs::File;
use std::io::{self, Read, Write};

use sha2::{Digest, Sha256};

use crate::{PackEntry, PackError, PackFile};

/// A size-capped writer that hashes bytes as they arrive.
pub(crate) struct HashingWriter {
    file: File,
    hash: Sha256,
    written: u64,
    limit: u64,
    exceeded: bool,
}

impl HashingWriter {
    /// Creates a writer for one declared file size.
    pub(crate) fn new(file: File, limit: u64) -> Self {
        Self {
            file,
            hash: Sha256::new(),
            written: 0,
            limit,
            exceeded: false,
        }
    }

    /// Returns whether the fetcher attempted to exceed the declared size.
    pub(crate) fn exceeded(&self) -> bool {
        self.exceeded
    }

    /// Flushes and checks the declared size and digest.
    ///
    /// # Errors
    /// Returns a size, hash, or I/O error.
    pub(crate) fn finish(
        &mut self,
        pack: &PackEntry,
        expected: &PackFile,
    ) -> Result<(), PackError> {
        self.file.flush().map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: expected.path.clone(),
        })?;
        if self.exceeded || self.written != expected.size_bytes {
            return Err(PackError::Size {
                id: pack.id.clone(),
                path: expected.path.clone(),
            });
        }
        if hex_digest(&self.hash.clone().finalize()) != expected.sha256 {
            return Err(PackError::HashMismatch {
                id: pack.id.clone(),
                path: expected.path.clone(),
            });
        }
        Ok(())
    }
}

impl Write for HashingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.written);
        if bytes.len() as u64 > remaining {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "declared file size exceeded",
            ));
        }
        let written = self.file.write(bytes)?;
        self.hash.update(&bytes[..written]);
        self.written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub(crate) fn hash_file(path: &std::path::Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex_digest(&hash.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}
