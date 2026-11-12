//! The preview: the change's documents as a virtual file set over the disk, readable by the
//! normal loader and `check`.

mod planned;
mod sheets;

pub use planned::{cache_mismatch, planned};

use crate::edit::Docs;
use crate::paths::{normalize, to_rel};
use anyhow::Result;
use brickie_check::check::{self, Loaded};
use brickie_check::config::{self, Config, FileSource};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

pub struct Preview {
    root: PathBuf,
    config: PathBuf,
    files: BTreeMap<String, String>,
    created: BTreeSet<String>,
}

impl Preview {
    pub(crate) fn new(root: &Path, config: &str, docs: &Docs) -> Preview {
        Preview {
            root: root.to_path_buf(),
            config: root.join(config),
            files: docs.texts(),
            created: docs.created().clone(),
        }
    }

    /// Every config file of the change (workspace-relative path -> text).
    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }

    /// Files the change creates.
    pub fn created(&self) -> &BTreeSet<String> {
        &self.created
    }

    /// Absolute path of the root config.
    pub fn config_path(&self) -> &Path {
        &self.config
    }

    /// Load and validate the previewed config tree.
    pub fn load_config(&self) -> Result<Config> {
        config::load_from(self, &self.config)
    }

    /// `check` over the previewed config tree and the measured cache.
    pub fn check(&self) -> Result<Loaded> {
        check::load_from(self, &self.config)
    }

    /// A message when the measured cache does not belong to this workspace.
    pub fn cache_mismatch(&self, loaded: &Loaded) -> Option<String> {
        cache_mismatch(&self.root, loaded)
    }

    fn get(&self, path: &Path) -> Option<&String> {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(path)
        };
        self.files.get(&to_rel(&self.root, &abs)?)
    }
}

impl FileSource for Preview {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.get(path) {
            Some(t) => Ok(t.clone()),
            None => std::fs::read_to_string(path),
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        self.get(path).is_some() || path.is_file()
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        match self.get(path) {
            Some(_) => Ok(normalize(path)),
            None => path.canonicalize(),
        }
    }
}
