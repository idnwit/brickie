//! The preview: documents in memory as a virtual file set over the disk, readable by the
//! normal loader, so a write is validated before any file is written.

mod planned;

pub use planned::{cache_mismatch, planned};

use crate::edit::Docs;
use crate::paths::{normalize, to_rel};
use anyhow::Result;
use brickie_check::config::{self, Config, FileSource};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub struct Preview {
    root: PathBuf,
    config: PathBuf,
    files: BTreeMap<String, String>,
}

impl Preview {
    pub(crate) fn new(root: &Path, config: &str, docs: &Docs) -> Preview {
        Preview {
            root: root.to_path_buf(),
            config: root.join(config),
            files: docs.texts(),
        }
    }

    /// Load and validate the previewed config tree.
    pub fn load_config(&self) -> Result<Config> {
        config::load_from(self, &self.config)
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
