//! `apply`: ops written to the draft (forked from the published head when there is none). Under
//! the lock: record a hand edit, check the base, build the new files in memory, load and
//! validate the whole design (config only, never against the code), store the draft, append
//! the version.

use super::commit::Stamp;
use super::tree::{self, Files};
use super::{Design, history, of_track, stale};
use crate::edit::{Docs, apply};
use crate::lock::{Lock, write_atomic};
use crate::preview::Preview;
use crate::report::diff::diff;
use crate::scaffold;
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::{By, Op, PlanAction, PlanFile, Task, Track, Version};
use std::collections::BTreeSet;

/// What a write did.
#[derive(Debug)]
pub struct Outcome {
    /// The head after the write (the old head when nothing changed).
    pub version: Version,
    /// `false`: the ops left every file as it was; no version was added.
    pub changed: bool,
    /// Code files scaffolded for new bricks (only when asked for).
    pub scaffold: Vec<PlanFile>,
    /// Facts about the ops that did not stop the write (a rule that changes nothing).
    pub notes: Vec<String>,
}

/// What `apply` would do.
#[derive(Debug)]
pub struct Plan {
    /// The semantic diff, as `design diff` prints it.
    pub diff: Vec<String>,
    /// Tasks the change adds to the todo list.
    pub new_todo: Vec<Task>,
    /// Code files that would be scaffolded.
    pub scaffold: Vec<PlanFile>,
    /// Facts about the ops that would not stop the write.
    pub notes: Vec<String>,
}

struct Prepared {
    entries: Vec<Version>,
    old: Files,
    new: Files,
    cfg: brickie_check::config::Config,
    notes: Vec<String>,
    /// No draft yet: the published head the write forks the draft from.
    fork: Option<u64>,
}

impl Design {
    /// Apply `ops` to the draft on top of version `base` (the draft head, or the published head
    /// when there is no draft yet), as one new draft version by `by`. Writes the draft only,
    /// never the published files or source code.
    pub fn apply(&self, base: u64, by: By, note: Option<String>, ops: Vec<Op>) -> Result<Outcome> {
        self.write(base, by, note, ops, false)
    }

    /// [`Design::apply`], and the code of the bricks the ops add is scaffolded too.
    pub fn apply_scaffold(
        &self,
        base: u64,
        by: By,
        note: Option<String>,
        ops: Vec<Op>,
    ) -> Result<Outcome> {
        self.write(base, by, note, ops, true)
    }

    fn write(
        &self,
        base: u64,
        by: By,
        note: Option<String>,
        ops: Vec<Op>,
        code: bool,
    ) -> Result<Outcome> {
        let lock = self.lock()?;
        let p = self.prepare(&lock, base, &ops)?;
        let notes = p.notes.clone();
        let stamp = Stamp {
            by,
            note,
            ops: ops.clone(),
            fork: p.fork,
            reverts: None,
        };
        let mut out = self.commit_draft(&p.entries, &p.old, &p.new, stamp)?;
        out.notes = notes;
        if out.changed {
            if code {
                out.scaffold = self.scaffold(&p.cfg, &ops)?;
            }
            out.scaffold.extend(tree::created(&p.old, &p.new));
        }
        Ok(out)
    }

    /// Run `apply` in memory only: check the base, apply and validate the ops, and report what
    /// would change. Nothing is written (a hand edit is still recorded, as on any read).
    pub fn dry_run(&self, base: u64, ops: Vec<Op>) -> Result<Plan> {
        self.plan(base, ops, false)
    }

    /// [`Design::dry_run`], including the code files `apply_scaffold` would write.
    pub fn dry_run_scaffold(&self, base: u64, ops: Vec<Op>) -> Result<Plan> {
        self.plan(base, ops, true)
    }

    fn plan(&self, base: u64, ops: Vec<Op>, code: bool) -> Result<Plan> {
        let lock = self.lock()?;
        let p = self.prepare(&lock, base, &ops)?;
        let before = Docs::from_base(&p.old, Some(self.root.clone()))?;
        let before = self.todo_of(&Preview::new(&self.root, &self.config, &before));
        let after = Docs::from_base(&p.new, Some(self.root.clone()))?;
        let after = self.todo_of(&Preview::new(&self.root, &self.config, &after));
        let new_todo = (after.into_iter())
            .filter(|t| !before.iter().any(|b| b.text == t.text))
            .collect();
        Ok(Plan {
            diff: diff(&p.old, &p.new),
            new_todo,
            scaffold: [
                match code {
                    true => self.scaffold_plan(&p.cfg, &ops),
                    false => Vec::new(),
                },
                tree::created(&p.old, &p.new),
            ]
            .concat(),
            notes: p.notes,
        })
    }

    /// Check the base, apply `ops` to the working files in memory and validate the design.
    fn prepare(&self, lock: &Lock, base: u64, ops: &[Op]) -> Result<Prepared> {
        if ops.is_empty() {
            bail!("nothing to apply");
        }
        let (entries, published) = self.sync(lock)?;
        let (old, fork) = self.work_base(&entries, published, base)?;
        let mut docs = Docs::from_base(&old, Some(self.root.clone()))?;
        let mut notes = Vec::new();
        for op in ops {
            if matches!(op, Op::AddRule { .. })
                && let Ok(cfg) = Preview::new(&self.root, &self.config, &docs).load_config()
            {
                notes.extend(self.rule_note(&cfg, op));
            }
            apply(&mut docs, op).with_context(|| format!("{} on {}", op.name(), op.file()))?;
        }
        let cfg = Preview::new(&self.root, &self.config, &docs)
            .load_config()
            .map_err(|e| anyhow::anyhow!("{DESIGN_INVALID}: {e:#}; nothing was written"))?;
        let new: Files = (docs.texts().into_iter())
            .map(|(k, v)| (k, v.into_bytes()))
            .collect();
        Ok(Prepared {
            entries,
            old,
            new,
            cfg,
            notes,
            fork,
        })
    }

    /// The files a write starts from and, with no draft yet, the published head it forks; the
    /// base must be the head of that track.
    pub(super) fn work_base(
        &self,
        entries: &[Version],
        published: Files,
        base: u64,
    ) -> Result<(Files, Option<u64>)> {
        match self.draft()? {
            Some(d) => {
                stale::check(&of_track(entries, Track::Draft), base)?;
                Ok((d.files, None))
            }
            None => {
                stale::check(&of_track(entries, Track::Published), base)?;
                Ok((published, Some(history::head(entries)?.v)))
            }
        }
    }

    /// The code files the ops' new bricks need (never overwrites a file); nothing is written.
    fn scaffold_plan(&self, cfg: &brickie_check::config::Config, ops: &[Op]) -> Vec<PlanFile> {
        // A composite made of an existing brick needs no code: its folder holds the new
        // `brickie.toml` already.
        let added: BTreeSet<String> = (ops.iter())
            .filter_map(|op| match op {
                Op::AddBrick { brick, .. } => Some(brick.name.clone()),
                _ => None,
            })
            .collect();
        match added.is_empty() {
            true => Vec::new(),
            false => scaffold::scaffold(&self.root, cfg, &added),
        }
    }

    /// Scaffold the code of the bricks the ops add (never overwrites a file).
    fn scaffold(&self, cfg: &brickie_check::config::Config, ops: &[Op]) -> Result<Vec<PlanFile>> {
        let plan = self.scaffold_plan(cfg, ops);
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
