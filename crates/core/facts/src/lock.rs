//! The cache lock: `.brickie/cache.lock`, held by whoever writes `cache.json` (`brickie measure`
//! or the server's measure job) for the whole run. A second writer fails at once with a clear
//! message instead of waiting. The file holds the holder's PID; a lock whose process is gone
//! (a crash) is stale and taken over.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Held while it lives; dropping it removes the lock file.
#[derive(Debug)]
pub struct CacheLock {
    path: PathBuf,
}

impl CacheLock {
    /// Take the lock of the cache directory `dir` (`.brickie`), creating the directory.
    pub fn acquire(dir: &Path) -> Result<CacheLock, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        let path = dir.join("cache.lock");
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    let _ = write!(f, "{}", std::process::id());
                    return Ok(CacheLock { path });
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    let holder = std::fs::read_to_string(&path).unwrap_or_default();
                    match holder.trim().parse::<u32>() {
                        Ok(pid) if alive(pid) => return Err(busy(&path, pid)),
                        // Stale (holder gone) or unreadable: take it over.
                        _ => {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
                Err(e) => return Err(format!("create {}: {e}", path.display())),
            }
        }
        Err(format!("could not take {}", path.display()))
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn busy(path: &Path, pid: u32) -> String {
    format!(
        "another measure (process {pid}) is writing the cache; wait for it to finish and retry \
         (lock {})",
        path.display()
    )
}

/// Whether process `pid` exists (`kill -0`); the current process always does.
fn alive(pid: u32) -> bool {
    pid == std::process::id()
        || Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_writer_fails_and_stale_lock_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().join(".brickie");
        let held = CacheLock::acquire(&d).unwrap();
        let err = CacheLock::acquire(&d).unwrap_err();
        assert!(err.contains("another measure"), "{err}");
        drop(held);
        assert!(!d.join("cache.lock").exists());
        // A lock left by a process that is gone is taken over.
        std::fs::write(d.join("cache.lock"), "4000000000").unwrap();
        let _l = CacheLock::acquire(&d).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.join("cache.lock")).unwrap(),
            std::process::id().to_string()
        );
    }
}
