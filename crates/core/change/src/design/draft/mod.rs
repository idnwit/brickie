//! The draft: the one design in progress, stored under `.brickie/draft/` as `draft.json`
//! (`{"forked_from": N}`) and `files/`, the config files by workspace-relative path (`..` stored
//! as `^`). Every design edit writes it; `check` reads it as the working design. The published tree changes only
//! by `publish` (and by hand). The draft is read through the preview overlay, so `path =` and
//! includes resolve as for the real files.

mod gate;
mod publish;
mod rebase;

pub use publish::Blocked;

use super::Design;
use super::history::{self, encode};
use super::tree::Files;
use super::working::Working;
use crate::lock::write_atomic;
use anyhow::{Context, Result};
use brickie_check::check::{self, Loaded};
use brickie_model::design::Track;
use std::path::{Path, PathBuf};

/// The draft on disk.
pub(crate) struct Draft {
    /// The published version it is based on.
    pub forked_from: u64,
    pub files: Files,
}

impl Design {
    fn draft_dir(&self) -> PathBuf {
        self.root.join(".brickie/draft")
    }

    /// The draft, when one exists.
    pub(crate) fn draft(&self) -> Result<Option<Draft>> {
        let meta = self.draft_dir().join("draft.json");
        let Ok(text) = std::fs::read_to_string(&meta) else {
            return Ok(None);
        };
        let v: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("parse {}", meta.display()))?;
        let forked_from = v["forked_from"]
            .as_u64()
            .with_context(|| format!("{} has no forked_from", meta.display()))?;
        let mut files = Files::new();
        let dir = self.draft_dir().join("files");
        walk(&dir, &dir, &mut files)?;
        Ok(Some(Draft { forked_from, files }))
    }

    /// The working tree from the design files alone (no measured code): the draft when one
    /// exists, else the published design. Design mode shows it.
    pub fn load_working(&self) -> Result<Loaded> {
        match self.draft()? {
            Some(d) => check::load_design(&self.preview(&d.files)?, &self.config_path()),
            None => check::load_design(&brickie_check::config::Disk, &self.config_path()),
        }
    }

    /// The working design checked against the measured code: the draft when one exists, else
    /// the published files. The one source of `check`, `interface` and the viewer. Reads only;
    /// the version names come from the history as recorded.
    pub fn load_checked(&self) -> Result<(Loaded, Working)> {
        let entries = self.history.entries()?;
        let (loaded, track, forked_from) = match self.draft()? {
            Some(d) => {
                let src = self.preview(&d.files)?;
                let l = check::load_from(&src, &self.config_path())?;
                (l, Track::Draft, Some(d.forked_from))
            }
            None => {
                let l = check::load(&self.config_path())?;
                (l, Track::Published, None)
            }
        };
        let v = history::head_of(&entries, track).map_or(0, |h| h.v);
        Ok((
            loaded,
            Working {
                track,
                v,
                forked_from,
            },
        ))
    }

    /// Replace the draft with `files`, based on published version `forked_from`.
    pub(super) fn store_draft(&self, forked_from: u64, files: &Files) -> Result<()> {
        let tmp = self.root.join(".brickie/draft.tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        for (rel, bytes) in files {
            write_atomic(&tmp.join("files").join(encode(rel)), bytes)?;
        }
        let meta = serde_json::json!({ "forked_from": forked_from });
        write_atomic(&tmp.join("draft.json"), meta.to_string().as_bytes())?;
        self.drop_draft()?;
        std::fs::rename(&tmp, self.draft_dir()).context("store .brickie/draft")
    }

    /// Remove the draft (nothing happens when there is none).
    pub(super) fn drop_draft(&self) -> Result<()> {
        match std::fs::remove_dir_all(self.draft_dir()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(e).context("remove .brickie/draft")
            }
            _ => Ok(()),
        }
    }
}

fn walk(base: &Path, dir: &Path, out: &mut Files) -> Result<()> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in read {
        let path = entry?.path();
        if path.is_dir() {
            walk(base, &path, out)?;
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .context("draft file outside the draft")?;
        let rel: Vec<String> = (rel.components())
            .map(|c| match c.as_os_str().to_string_lossy() {
                s if s == "^" => "..".to_string(),
                s => s.into_owned(),
            })
            .collect();
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        out.insert(rel.join("/"), bytes);
    }
    Ok(())
}
