//! Content hashing.

use sha2::{Digest, Sha256};

/// The SHA-256 of `bytes` as 64 lowercase hex digits; the content hash of source and config files.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
