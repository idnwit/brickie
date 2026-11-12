//! Publish: write the change's config files, then scaffold the code of new bricks. Refuses
//! while an op is proposed or in conflict, when a touched file was edited outside brickie,
//! or when the result does not load. Existing files are never overwritten.

mod crates;
mod scaffold;

use crate::session::{Change, materialize, replay};
use anyhow::{Context, Result, bail};
use brickie_model::change::{Op, Plan, PlanAction, PlanFile, State};
use brickie_model::codes::{CHANGE_CONFLICT, CHANGE_INVALID, CHANGE_OUTSIDE_EDIT, CHANGE_PROPOSED};
use std::collections::BTreeSet;

impl Change {
    /// Every file publish would write, create or edit, and why it would refuse.
    pub fn publish_plan(&self) -> Result<Plan> {
        let mut plan = Plan::default();
        let log = self.log()?;
        for o in &log.ops {
            if let Some(c) = &o.conflict {
                plan.blockers
                    .push(format!("{CHANGE_CONFLICT}: op {}: {c}", o.seq));
            } else if o.state == State::Proposed {
                plan.blockers.push(format!(
                    "{CHANGE_PROPOSED}: op {} ({} {}) is still proposed",
                    o.seq,
                    o.op.name(),
                    o.key
                ));
            }
        }
        let base = self.store.base()?;
        let mut ops = materialize(&log.entries);
        let docs = replay(&base, None, &mut ops, false)?;
        let preview = self.preview(false)?;
        for (rel, text) in docs.texts() {
            let old = base.get(&rel);
            if old.is_some_and(|b| String::from_utf8_lossy(b) == text) {
                continue;
            }
            let disk = std::fs::read(self.root.join(&rel)).ok();
            if disk.as_ref() != old {
                plan.blockers.push(format!(
                    "{CHANGE_OUTSIDE_EDIT}: {rel} was edited outside brickie since the change was opened"
                ));
            }
            let action = if old.is_some() {
                PlanAction::Write
            } else {
                PlanAction::Create
            };
            plan.files.push(PlanFile {
                path: rel,
                action,
                note: "config".into(),
                content: Some(text),
            });
        }
        plan.files.extend(docs.removed().iter().map(|rel| PlanFile {
            path: rel.clone(),
            action: PlanAction::Skip,
            note: "no longer part of the design (ungrouped); delete it by hand".into(),
            content: None,
        }));
        plan.warnings = self.reason_warnings()?;
        match preview.load_config() {
            Ok(cfg) => {
                let (added, moved) = added_bricks(&ops);
                plan.files
                    .extend(scaffold::scaffold(&self.root, &cfg, &added));
                plan.files.extend(moved.into_iter().map(|name| PlanFile {
                    path: format!("brick:{name}"),
                    action: PlanAction::Skip,
                    note:
                        "moved in the design; move its code by hand (publish moves no code)".into(),
                    content: None,
                }));
            }
            Err(e) => plan.blockers.push(format!("{CHANGE_INVALID}: {e:#}")),
        }
        Ok(plan)
    }

    /// Run the plan and close the change.
    pub fn publish(self) -> Result<Plan> {
        let _lock = self.store.lock()?;
        let plan = self.publish_plan()?;
        if !plan.blockers.is_empty() {
            bail!("publish refused:\n  {}", plan.blockers.join("\n  "));
        }
        for f in &plan.files {
            self.write(f)?;
        }
        drop(_lock);
        self.store.remove()?;
        Ok(plan)
    }

    fn write(&self, f: &PlanFile) -> Result<()> {
        let path = self.root.join(&f.path);
        let create = |p: &std::path::Path| {
            p.parent()
                .map(std::fs::create_dir_all)
                .transpose()
                .with_context(|| format!("create {}", p.display()))
        };
        match (f.action, &f.content) {
            (PlanAction::Mkdir, _) => {
                std::fs::create_dir_all(&path).with_context(|| format!("create {}", path.display()))
            }
            (PlanAction::Create, Some(c)) if !path.exists() => {
                create(&path)?;
                std::fs::write(&path, c).with_context(|| format!("write {}", path.display()))
            }
            (PlanAction::Write | PlanAction::Edit, Some(c)) => {
                crate::store::write_atomic(&path, c.as_bytes())
            }
            _ => Ok(()),
        }
    }
}

/// Names of the bricks the accepted ops add or make composite (their code may not exist yet),
/// and the bricks they move (existing code is not moved: a manual step in the plan).
fn added_bricks(ops: &[brickie_model::change::OpRecord]) -> (BTreeSet<String>, Vec<String>) {
    let live = ops
        .iter()
        .filter(|o| o.state == State::Accepted && o.conflict.is_none());
    let (mut added, mut moved) = (BTreeSet::new(), Vec::new());
    for o in live {
        match &o.op {
            Op::AddBrick { brick, .. } => {
                added.insert(brick.name.clone());
            }
            Op::MakeComposite { name, .. } => {
                added.insert(name.clone());
            }
            Op::RenameBrick { from, to, .. } if added.remove(from) => {
                added.insert(to.clone());
            }
            Op::MoveBrick { name, .. } if !added.contains(name) => moved.push(name.clone()),
            _ => {}
        }
    }
    (added, moved)
}
