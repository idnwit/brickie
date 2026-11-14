//! What exists in the workspace: packages (from the cache, else none) and file contents with
//! pending edits.

use crate::paths::to_rel;
use brickie_facts::Cache;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Workspace {
    pub root: PathBuf,
    /// Measured crates by package name and crate id -> (package dir, crate root file),
    /// workspace-relative.
    pub packages: BTreeMap<String, (String, String)>,
    /// Pending edits of existing files (path -> new content).
    pub edits: BTreeMap<String, String>,
}

impl Workspace {
    pub fn read(root: &Path) -> Workspace {
        let cache: Option<Cache> = std::fs::read(root.join(".brickie/cache.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let mut packages = BTreeMap::new();
        for c in cache.iter().flat_map(|c| &c.crates) {
            let src = Path::new(&c.root);
            let src = match src.is_absolute() {
                true => to_rel(root, src).unwrap_or_default(),
                false => c.root.clone(),
            };
            for name in [&c.package, &c.id] {
                if c.kind == "lib" || !packages.contains_key(name) {
                    packages.insert(name.clone(), (c.dir.clone(), src.clone()));
                }
            }
        }
        Workspace {
            root: root.to_path_buf(),
            packages,
            edits: BTreeMap::new(),
        }
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.root.join(rel).exists()
    }

    /// Current content of an existing file, pending edits included.
    pub fn content(&mut self, rel: &str) -> Option<&mut String> {
        if !self.edits.contains_key(rel) {
            let text = std::fs::read_to_string(self.root.join(rel)).ok()?;
            self.edits.insert(rel.to_string(), text);
        }
        self.edits.get_mut(rel)
    }
}
