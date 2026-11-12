//! A change session: open, record ops, review them, read what is new, preview and discard.
//! `publish` lives in `crate::publish`, reports in `crate::report`.

mod design;
mod record;
mod replay;

pub use replay::{materialize, replay};

use crate::preview::Preview;
use crate::store::{self, Store};
use anyhow::{Context, Result, bail};
use brickie_model::change::{By, Entry, OpRecord};
use brickie_model::codes::{CHANGE_NONE, CHANGE_OPEN};
use std::path::{Path, PathBuf};

/// The open change of one config tree.
pub struct Change {
    /// Canonical directory of the root config.
    pub(crate) root: PathBuf,
    /// File name of the root config (relative to `root`).
    pub(crate) config: String,
    pub(crate) store: Store,
}

/// The log: every entry, and the op records with review state and conflicts (replayed with
/// the proposed ops).
pub struct Log {
    pub entries: Vec<Entry>,
    pub ops: Vec<OpRecord>,
}

/// Entries after a reader's cursor.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Since {
    /// The cursor the entries start after.
    pub from: u64,
    /// The last `seq` (the new cursor when advanced).
    pub head: u64,
    pub entries: Vec<Entry>,
}

fn locate(config: &Path) -> Result<(PathBuf, String)> {
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
    Ok((root, name))
}

impl Change {
    /// Whether a change is open for the config.
    pub fn is_open(config: &Path) -> bool {
        locate(config).is_ok_and(|(root, _)| Store::at(&root).exists())
    }

    /// Open a new change: snapshot the config tree into `base/`.
    pub fn open(config: &Path, intent: &str) -> Result<Change> {
        let (root, name) = locate(config)?;
        let store = Store::at(&root);
        if store.exists() {
            bail!("{CHANGE_OPEN}: a change is already open (publish or discard it first)");
        }
        if !root.join(&name).is_file() {
            bail!("read {}: no such file", config.display());
        }
        store.create(intent, &store::snapshot(&root, &name)?)?;
        Ok(Change {
            root,
            config: name,
            store,
        })
    }

    /// The open change of the config.
    pub fn load(config: &Path) -> Result<Change> {
        let (root, name) = locate(config)?;
        let store = Store::at(&root);
        if !store.exists() {
            bail!("{CHANGE_NONE}: no change is open (run `brickie change open --intent ...`)");
        }
        Ok(Change {
            root,
            config: name,
            store,
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

    pub fn intent(&self) -> Result<String> {
        self.store.intent()
    }

    pub fn log(&self) -> Result<Log> {
        let entries = self.store.entries()?;
        let mut ops = materialize(&entries);
        replay(&self.store.base()?, None, &mut ops, true)?;
        Ok(Log { entries, ops })
    }

    /// The design with the accepted ops (and the proposed ones when `proposed`) applied.
    pub fn preview(&self, proposed: bool) -> Result<Preview> {
        let mut ops = materialize(&self.store.entries()?);
        let docs = replay(&self.store.base()?, None, &mut ops, proposed)?;
        Ok(Preview::new(&self.root, &self.config, &docs))
    }

    /// Entries after `since`, or after the reader's cursor (which then moves to the head).
    pub fn since(&self, reader: By, since: Option<u64>) -> Result<Since> {
        let entries = self.store.entries()?;
        let head = entries.last().map_or(0, Entry::seq);
        let from = match since {
            Some(s) => s,
            None => self.store.readers()?.get(&reader).copied().unwrap_or(0),
        };
        if since.is_none() {
            self.store.set_reader(reader, head)?;
        }
        let entries = entries.into_iter().filter(|e| e.seq() > from).collect();
        Ok(Since {
            from,
            head,
            entries,
        })
    }

    /// Delete the change; the config files are untouched.
    pub fn discard(self) -> Result<()> {
        let _lock = self.store.lock()?;
        self.store.remove()
    }
}
