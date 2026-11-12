//! The virtual file set: workspace-relative path -> parsed document.

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use toml_edit::DocumentMut;

pub struct Docs {
    files: BTreeMap<String, DocumentMut>,
    created: BTreeSet<String>,
    /// Base files an op dropped from the design (`ungroup`); publish never deletes them.
    removed: BTreeSet<String>,
    /// The root config's directory: a file that exists there cannot be created by an op.
    root: Option<PathBuf>,
}

impl Docs {
    pub fn from_base(base: &BTreeMap<String, Vec<u8>>, root: Option<PathBuf>) -> Result<Docs> {
        let mut files = BTreeMap::new();
        for (rel, bytes) in base {
            let text = String::from_utf8_lossy(bytes);
            let doc: DocumentMut = text.parse().with_context(|| format!("parse {rel}"))?;
            files.insert(rel.clone(), doc);
        }
        Ok(Docs {
            files,
            created: BTreeSet::new(),
            removed: BTreeSet::new(),
            root,
        })
    }

    pub fn get(&self, file: &str) -> Option<&DocumentMut> {
        self.files.get(file)
    }

    pub fn get_mut(&mut self, file: &str) -> Result<&mut DocumentMut> {
        self.files
            .get_mut(file)
            .with_context(|| format!("{file} is not a config file of this change"))
    }

    /// A new, empty config file.
    pub fn create(&mut self, file: &str) -> Result<&mut DocumentMut> {
        let on_disk = self.root.as_ref().is_some_and(|r| r.join(file).exists());
        let dropped = self.removed.remove(file);
        if self.files.contains_key(file) || (on_disk && !dropped) {
            bail!("{file} already exists");
        }
        if !dropped {
            self.created.insert(file.to_string());
        }
        Ok(self.files.entry(file.to_string()).or_default())
    }

    /// Drop a file from the design. A file an op created is simply gone; a base file is
    /// remembered in `removed`.
    pub fn remove(&mut self, file: &str) {
        self.files.remove(file);
        if !self.created.remove(file) {
            self.removed.insert(file.to_string());
        }
    }

    /// Base files dropped from the design.
    pub fn removed(&self) -> &BTreeSet<String> {
        &self.removed
    }

    /// Files created by ops (not in the base).
    pub fn created(&self) -> &BTreeSet<String> {
        &self.created
    }

    /// Every file as text.
    pub fn texts(&self) -> BTreeMap<String, String> {
        self.files
            .iter()
            .map(|(k, d)| (k.clone(), d.to_string()))
            .collect()
    }
}
