//! Writes: `apply` (ops) and `undo` (a version's file changes reverted). Both run under the
//! lock: record a hand edit, check the base, build the new files in memory, load and validate
//! the whole design, write each changed file by rename, append the version.

use super::tree::{self, Files};
use super::{Design, history, stale};
use crate::edit::{Docs, apply};
use crate::lock::write_atomic;
use crate::preview::Preview;
use crate::report::diff::diff;
use crate::scaffold;
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::{By, Op, PlanAction, PlanFile, Version};
use std::collections::BTreeSet;

/// What a write did.
#[derive(Debug)]
pub struct Outcome {
    /// The head after the write (the old head when nothing changed).
    pub version: Version,
    /// `false`: the ops left every file as it was; no version was added.
    pub changed: bool,
    /// Code files scaffolded for new bricks.
    pub scaffold: Vec<PlanFile>,
}

impl Design {
    /// Apply `ops` on top of version `base`, as one new version by `by`.
    pub fn apply(&self, base: u64, by: By, note: Option<String>, ops: Vec<Op>) -> Result<Outcome> {
        if ops.is_empty() {
            bail!("nothing to apply");
        }
        let lock = self.lock()?;
        let (entries, old) = self.sync(&lock)?;
        stale::check(&entries, base)?;
        let mut docs = Docs::from_base(&old, Some(self.root.clone()))?;
        for op in &ops {
            apply(&mut docs, op).with_context(|| format!("{} on {}", op.name(), op.file()))?;
        }
        let cfg = Preview::new(&self.root, &self.config, &docs)
            .load_config()
            .map_err(|e| anyhow::anyhow!("{DESIGN_INVALID}: {e:#}; nothing was written"))?;
        let new: Files = (docs.texts().into_iter())
            .map(|(k, v)| (k, v.into_bytes()))
            .collect();
        let mut out = self.commit(&entries, &old, &new, by, note, ops.clone())?;
        if out.changed {
            out.scaffold = self.scaffold(&cfg, &ops)?;
        }
        Ok(out)
    }

    /// Revert the file changes of version `target` (default: the head) as a new version. Refused
    /// when a file it changed is no longer as it left it (a later version changed it and was not
    /// reverted), and for an agent when `target` was made by a human or by hand.
    pub fn undo(&self, base: u64, by: By, target: Option<u64>) -> Result<Outcome> {
        let lock = self.lock()?;
        let (entries, old) = self.sync(&lock)?;
        stale::check(&entries, base)?;
        let n = target.unwrap_or(history::head(&entries)?.v);
        let t = (entries.iter())
            .find(|e| e.v == n)
            .with_context(|| format!("v{n} is not in the history"))?;
        if t.full {
            bail!("v{n} is the oldest kept version; there is nothing before it to go back to");
        }
        if by == By::Agent && t.by != By::Agent {
            bail!(
                "v{n} was made by {}; an agent never undoes it (ask the human)",
                t.by.name()
            );
        }
        let touched: BTreeSet<&String> = t.files.iter().chain(&t.removed).collect();
        let after = self.history.state(&entries, n)?;
        // A later version that changed a file and was reverted since leaves it as `n` did.
        if let Some(f) = touched.iter().find(|f| old.get(**f) != after.get(**f)) {
            let later = (entries.iter().rev())
                .find(|e| e.v > n && e.files.iter().chain(&e.removed).any(|x| x == *f));
            let v = later.map_or(n, |e| e.v);
            bail!("v{v} changed {f} after v{n}; undo v{v} first");
        }
        let before = self.history.state(&entries, n - 1)?;
        let mut new = old.clone();
        for f in touched {
            match before.get(f) {
                Some(b) => new.insert(f.clone(), b.clone()),
                None => new.remove(f),
            };
        }
        let docs = Docs::from_base(&new, None)?;
        Preview::new(&self.root, &self.config, &docs)
            .load_config()
            .map_err(|e| anyhow::anyhow!("{DESIGN_INVALID}: {e:#}; nothing was written"))?;
        self.commit(
            &entries,
            &old,
            &new,
            by,
            Some(format!("undo v{n}")),
            Vec::new(),
        )
    }

    /// Write the files that differ from `old`, delete the ones gone, append the version.
    fn commit(
        &self,
        entries: &[Version],
        old: &Files,
        new: &Files,
        by: By,
        note: Option<String>,
        ops: Vec<Op>,
    ) -> Result<Outcome> {
        let head = history::head(entries)?;
        let changed: Vec<String> = (new.iter())
            .filter(|(p, b)| old.get(*p) != Some(*b))
            .map(|(p, _)| p.clone())
            .collect();
        let gone: Vec<String> = old
            .keys()
            .filter(|p| !new.contains_key(*p))
            .cloned()
            .collect();
        if changed.is_empty() && gone.is_empty() {
            return Ok(Outcome {
                version: head.clone(),
                changed: false,
                scaffold: Vec::new(),
            });
        }
        for p in &changed {
            write_atomic(&self.root.join(p), &new[p])?;
        }
        // A file this write created and an undo takes away; a file only dropped from the tree
        // (ungroup) stays on disk.
        for p in gone
            .iter()
            .filter(|p| ops.is_empty() && !p.ends_with(".review.toml"))
        {
            let _ = std::fs::remove_file(self.root.join(p));
        }
        let disk = tree::read(&self.root, &self.config)?;
        let version = Version {
            v: head.v + 1,
            hash: tree::hash(&disk),
            prev: Some(head.hash.clone()),
            by,
            at: stale::now(),
            note: note.filter(|n| !n.trim().is_empty()),
            ops,
            diff: diff(old, &disk),
            files: changed
                .into_iter()
                .filter(|p| disk.contains_key(p))
                .collect(),
            removed: old
                .keys()
                .filter(|p| !disk.contains_key(*p))
                .cloned()
                .collect(),
            full: false,
        };
        self.history.append(&version, &disk)?;
        Ok(Outcome {
            version,
            changed: true,
            scaffold: Vec::new(),
        })
    }

    /// Scaffold the code of the bricks the ops add (never overwrites a file).
    fn scaffold(&self, cfg: &brickie_check::config::Config, ops: &[Op]) -> Result<Vec<PlanFile>> {
        let mut added = BTreeSet::new();
        // A composite made of an existing brick needs no code: its folder holds the new
        // `brickie.toml` already.
        for op in ops {
            if let Op::AddBrick { brick, .. } = op {
                added.insert(brick.name.clone());
            }
        }
        if added.is_empty() {
            return Ok(Vec::new());
        }
        let plan = scaffold::scaffold(&self.root, cfg, &added);
        for f in &plan {
            let path = self.root.join(&f.path);
            match (f.action, &f.content) {
                (PlanAction::Mkdir, _) => std::fs::create_dir_all(&path)
                    .with_context(|| format!("create {}", path.display()))?,
                (PlanAction::Create, Some(c)) if !path.exists() => {
                    write_atomic(&path, c.as_bytes())?
                }
                (PlanAction::Edit, Some(c)) => write_atomic(&path, c.as_bytes())?,
                _ => {}
            }
        }
        Ok(plan)
    }
}
