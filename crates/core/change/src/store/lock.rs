//! The writer lock of a change: a file created exclusively, removed on drop.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Lock(PathBuf);

/// A lock older than this is left over from a crashed writer.
const STALE: Duration = Duration::from_secs(30);
const WAIT: Duration = Duration::from_secs(5);

impl Lock {
    pub fn acquire(path: &Path) -> Result<Lock> {
        let start = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(_) => return Ok(Lock(path.to_path_buf())),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e).with_context(|| format!("lock {}", path.display())),
            }
            if is_stale(path) {
                let _ = std::fs::remove_file(path);
                continue;
            }
            if start.elapsed() > WAIT {
                bail!("{} is held by another writer", path.display());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn is_stale(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age > STALE)
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Write `bytes` to `path` through a temporary file and a rename.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))
}
