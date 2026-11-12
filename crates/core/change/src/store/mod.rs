//! `.brickie/change/`: `base/` (raw bytes of the config tree at open), `ops.json` (the log,
//! append-only), `intent`, `readers.json` (last `seq` read per reader), `layout.json`
//! (canvas positions, UI only) and `lock`. Writers
//! take the lock, assign `seq` under it and write by atomic rename.

mod base;
mod lock;

pub use base::snapshot;
pub use lock::write_atomic;

use anyhow::{Context, Result};
use brickie_model::change::{By, Entry, LayoutHint};
use lock::Lock;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// The store of the config whose directory is `root`.
    pub fn at(root: &Path) -> Store {
        Store {
            dir: root.join(".brickie/change"),
        }
    }

    pub fn exists(&self) -> bool {
        self.dir.join("intent").is_file()
    }

    pub fn create(&self, intent: &str, base: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("create {}", self.dir.display()))?;
        let _lock = self.lock()?;
        for (rel, bytes) in base {
            base::write(&self.dir.join("base"), rel, bytes)?;
        }
        write_atomic(&self.dir.join("ops.json"), b"[]\n")?;
        write_atomic(&self.dir.join("readers.json"), b"{}\n")?;
        write_atomic(&self.dir.join("intent"), intent.as_bytes())
    }

    pub fn intent(&self) -> Result<String> {
        std::fs::read_to_string(self.dir.join("intent")).context("read the change intent")
    }

    pub fn base(&self) -> Result<BTreeMap<String, Vec<u8>>> {
        base::read(&self.dir.join("base"))
    }

    /// Add a file that was not part of the tree at open (named later by an op).
    pub fn add_base(&self, rel: &str, bytes: &[u8]) -> Result<()> {
        base::write(&self.dir.join("base"), rel, bytes)
    }

    pub fn entries(&self) -> Result<Vec<Entry>> {
        let path = self.dir.join("ops.json");
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
    }

    /// Under the lock: read the log, build the next entry from it (`make` gets the log and the
    /// next `seq`), append it.
    pub fn append(&self, make: impl FnOnce(&[Entry], u64) -> Result<Entry>) -> Result<Entry> {
        let _lock = self.lock()?;
        let mut entries = self.entries()?;
        let seq = entries.last().map_or(1, |e| e.seq() + 1);
        let entry = make(&entries, seq)?;
        entries.push(entry.clone());
        let json = serde_json::to_vec_pretty(&entries)?;
        write_atomic(&self.dir.join("ops.json"), &json)?;
        Ok(entry)
    }

    pub fn readers(&self) -> Result<BTreeMap<By, u64>> {
        let bytes = std::fs::read(self.dir.join("readers.json")).unwrap_or_default();
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    }

    pub fn set_reader(&self, reader: By, seq: u64) -> Result<()> {
        let _lock = self.lock()?;
        let mut r = self.readers()?;
        r.insert(reader, seq);
        write_atomic(&self.dir.join("readers.json"), &serde_json::to_vec(&r)?)
    }

    pub fn layout(&self) -> Result<BTreeMap<String, LayoutHint>> {
        let bytes = std::fs::read(self.dir.join("layout.json")).unwrap_or_default();
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    }

    pub fn set_layout(&self, brick: &str, hint: Option<LayoutHint>) -> Result<()> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        match hint {
            Some(h) => l.insert(brick.to_string(), h),
            None => l.remove(brick),
        };
        write_atomic(&self.dir.join("layout.json"), &serde_json::to_vec(&l)?)
    }

    pub fn lock(&self) -> Result<Lock> {
        Lock::acquire(&self.dir.join("lock"))
    }

    pub fn remove(&self) -> Result<()> {
        std::fs::remove_dir_all(&self.dir).with_context(|| format!("remove {}", self.dir.display()))
    }
}
