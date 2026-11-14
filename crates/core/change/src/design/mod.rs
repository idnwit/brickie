//! The design: the `brickie.toml` tree on disk, versioned. Every write is applied to the files
//! at once, under `.brickie/design.lock`, and based on a version (`base`): a write based on an
//! older version than the head is refused ([`Stale`]). A hand edit of the files becomes an
//! `external` version on the next read or write.

mod history;
mod links;
mod resolve;
pub use resolve::brick_path;
mod stale;
mod tree;
mod write;

pub use history::CAP;
pub use links::Link;
pub use stale::{Stale, ago};
pub use tree::Files;
pub use write::Outcome;

use crate::lock::Lock;
use crate::report::diff::diff;
use anyhow::{Context, Result};
use brickie_model::design::{By, Version};
use history::History;
use std::path::{Path, PathBuf};

pub struct Design {
    /// Canonical directory of the root config.
    root: PathBuf,
    /// File name of the root config (relative to `root`).
    config: String,
    history: History,
}

impl Design {
    /// The design whose root config is `config`.
    pub fn at(config: &Path) -> Result<Design> {
        let dir = config
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let root = dir
            .canonicalize()
            .with_context(|| format!("read {}", dir.display()))?;
        let name = config
            .file_name()
            .context("the config path has no file name")?
            .to_string_lossy()
            .into_owned();
        if !root.join(&name).is_file() {
            anyhow::bail!("read {}: no such file", config.display());
        }
        Ok(Design {
            history: History::at(&root),
            root,
            config: name,
        })
    }

    /// Canonical directory of the root config.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute path of the root config.
    pub fn config_path(&self) -> PathBuf {
        self.root.join(&self.config)
    }

    /// File name of the root config.
    pub fn config_name(&self) -> &str {
        &self.config
    }

    fn lock(&self) -> Result<Lock> {
        std::fs::create_dir_all(self.root.join(".brickie"))?;
        Lock::acquire(&self.root.join(".brickie/design.lock"))
    }

    /// The config files on disk now.
    pub fn files(&self) -> Result<Files> {
        tree::read(&self.root, &self.config)
    }

    /// Under the lock: record the first version, or an `external` one when the files differ
    /// from the head. Returns the history and the files on disk.
    fn sync(&self, _lock: &Lock) -> Result<(Vec<Version>, Files)> {
        let mut entries = self.history.entries()?;
        let files = self.files()?;
        let hash = tree::hash(&files);
        let v = match entries.last() {
            Some(h) if h.hash == hash => return Ok((entries, files)),
            Some(h) => {
                let old = self.history.state(&entries, h.v)?;
                let changed = files.iter().filter(|(p, b)| old.get(*p) != Some(*b));
                Version {
                    v: h.v + 1,
                    hash,
                    prev: Some(h.hash.clone()),
                    by: By::External,
                    at: stale::now(),
                    note: Some("edited outside brickie".into()),
                    ops: Vec::new(),
                    diff: diff(&old, &files),
                    files: changed.map(|(p, _)| p.clone()).collect(),
                    removed: old
                        .keys()
                        .filter(|p| !files.contains_key(*p))
                        .cloned()
                        .collect(),
                    full: false,
                }
            }
            None => Version {
                v: 1,
                hash,
                prev: None,
                by: By::External,
                at: stale::now(),
                note: Some("first version".into()),
                ops: Vec::new(),
                diff: Vec::new(),
                files: files.keys().cloned().collect(),
                removed: Vec::new(),
                full: true,
            },
        };
        self.history.append(&v, &files)?;
        entries.push(v);
        Ok((entries, files))
    }

    /// The head version (a hand edit is recorded first).
    pub fn head(&self) -> Result<Version> {
        let lock = self.lock()?;
        let (entries, _) = self.sync(&lock)?;
        Ok(history::head(&entries)?.clone())
    }

    /// Every kept version, oldest first.
    pub fn log(&self) -> Result<Vec<Version>> {
        let lock = self.lock()?;
        Ok(self.sync(&lock)?.0)
    }

    /// The config files at version `v`.
    pub fn state(&self, v: u64) -> Result<Files> {
        let entries = self.log()?;
        self.history.state(&entries, v)
    }

    /// The semantic diff from version `a` to `b`.
    pub fn diff(&self, a: u64, b: u64) -> Result<Vec<String>> {
        let entries = self.log()?;
        let (x, y) = (
            self.history.state(&entries, a)?,
            self.history.state(&entries, b)?,
        );
        Ok(diff(&x, &y))
    }
}
