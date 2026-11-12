//! Restores `Cargo.lock` after cargo or rust-analyzer touched it.

use std::path::{Path, PathBuf};

/// Restores `Cargo.lock` on drop if cargo or rust-analyzer changed (or created) it.
pub struct LockGuard {
    path: PathBuf,
    original: Option<Vec<u8>>,
}

impl LockGuard {
    pub fn new(dir: &Path) -> Self {
        let path = dir.join("Cargo.lock");
        let original = std::fs::read(&path).ok();
        LockGuard { path, original }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let now = std::fs::read(&self.path).ok();
        if now != self.original {
            let _ = match &self.original {
                Some(b) => std::fs::write(&self.path, b),
                None => std::fs::remove_file(&self.path),
            };
        }
    }
}
