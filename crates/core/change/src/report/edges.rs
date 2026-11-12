//! Rules the change adds (in the design with the proposed ops, not in the design as opened):
//! the planned edges (no measured module edge backs them yet) and the ones without a reason.

use crate::edit::{Docs, canonical};
use crate::paths::to_rel;
use crate::session::{Change, materialize, replay};
use anyhow::Result;
use brickie_check::check::Loaded;
use brickie_check::config::{Action, Config, DepRule, Scope, level_name};
use brickie_model::change::{OpRecord, PlannedEdge, State};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// `(file, index)` -> canonical TOML of each rule the change adds.
type Added = BTreeMap<(String, usize), String>;

/// A rule of a level with its physical file and index among that file's rules.
struct At<'a> {
    level: &'a Config,
    file: String,
    index: usize,
    rule: &'a DepRule,
}

fn rule_count(docs: &Docs, file: &str) -> usize {
    (0..)
        .take_while(|i| canonical(docs, file, &format!("rule:{i}")).is_some())
        .count()
}

/// Rules of `docs` whose canonical text is not among the base rules of the same file.
fn added(base: &Docs, docs: &Docs) -> Added {
    let mut out = Added::new();
    for file in docs.texts().keys() {
        let mut old: Vec<String> = (0..rule_count(base, file))
            .filter_map(|i| canonical(base, file, &format!("rule:{i}")))
            .collect();
        for i in 0..rule_count(docs, file) {
            let Some(c) = canonical(docs, file, &format!("rule:{i}")) else {
                continue;
            };
            match old.iter().position(|o| *o == c) {
                Some(p) => {
                    old.remove(p);
                }
                None => {
                    out.insert((file.clone(), i), c);
                }
            }
        }
    }
    out
}

/// Every rule of the tree with its file and index (a preset's rules once per level).
fn rules<'a>(cfg: &'a Config, root: &Path, out: &mut Vec<At<'a>>) {
    let mut by_file: BTreeMap<String, Vec<(u32, &DepRule)>> = BTreeMap::new();
    for r in &cfg.deps.rules {
        let (f, line) = r.loc.rsplit_once(':').unwrap_or((&r.loc, "0"));
        let Some(rel) = to_rel(root, &std::path::absolute(f).unwrap_or_default()) else {
            continue;
        };
        by_file
            .entry(rel)
            .or_default()
            .push((line.parse().unwrap_or(0), r));
    }
    for (file, mut list) in by_file {
        list.sort_by_key(|(line, _)| *line);
        for (index, (_, rule)) in list.into_iter().enumerate() {
            out.push(At {
                level: cfg,
                file: file.clone(),
                index,
                rule,
            });
        }
    }
    for b in &cfg.bricks {
        if let Some(sub) = &b.sub {
            rules(sub, root, out);
        }
    }
}

/// Measured module edges lifted to their level: `(level path, from, to, kind, scope)`.
type Lifted = BTreeSet<(Vec<String>, String, String, String, String)>;

fn lifted(l: &Loaded) -> Lifted {
    let mut out = Lifted::new();
    for e in l.cache.edges() {
        let (Some(p1), Some(p2)) = (l.resolved.path(&e.from), l.resolved.path(&e.to)) else {
            continue;
        };
        let k = p1.iter().zip(p2).take_while(|(a, b)| a == b).count();
        if let (Some(a), Some(b)) = (p1.get(k), p2.get(k)) {
            out.insert((p1[..k].to_vec(), a.clone(), b.clone(), e.kind, e.scope));
        }
    }
    out
}

fn backed(at: &At, measured: &Lifted) -> bool {
    let (lv, r) = (at.level, at.rule);
    measured.iter().any(|(path, from, to, kind, scope)| {
        let (Some(f), Some(t)) = (lv.brick(from), lv.brick(to)) else {
            return false;
        };
        *path == lv.path
            && r.from.specificity(f).is_some()
            && r.to.specificity(t).is_some()
            && (r.kinds.is_empty() || r.kinds.contains(kind))
            && r.scope.matches(scope)
    })
}

fn scope(s: Scope) -> String {
    match s {
        Scope::Prod => "prod",
        Scope::Test => "test",
        Scope::All => "all",
    }
    .into()
}

fn edge(at: &At, canon: &str, ops: &[OpRecord]) -> PlannedEdge {
    let r = at.rule;
    let op = ops.iter().rev().find(|o| {
        o.file == at.file
            && o.state != State::Rejected
            && o.conflict.is_none()
            && o.after.as_deref() == Some(canon)
    });
    PlannedEdge {
        level: level_name(&at.level.path),
        file: at.file.clone(),
        rule: at.index,
        from: r.from.to_string(),
        to: r.to.to_string(),
        kinds: r.kinds.clone(),
        scope: scope(r.scope),
        action: if r.action == Action::Allow {
            "allow"
        } else {
            "deny"
        }
        .into(),
        reason: r.reason.clone().filter(|s| !s.trim().is_empty()),
        help: r.help.clone(),
        seq: op.map(|o| o.seq),
        by: op.map(|o| o.by),
        state: op.map(|o| o.state),
    }
}

impl Change {
    /// The rules the change adds as edges, with `cfg` the previewed design; with `measured`
    /// only the ones no measured edge backs.
    fn added_edges(&self, cfg: &Config, measured: Option<&Lifted>) -> Result<Vec<PlannedEdge>> {
        let log = self.log()?;
        let base = self.store.base()?;
        let docs = replay(&base, None, &mut materialize(&log.entries), true)?;
        let added = added(&Docs::from_base(&base, None)?, &docs);
        let mut all = Vec::new();
        rules(cfg, &self.root, &mut all);
        let out = all.iter().filter_map(|at| {
            let canon = added.get(&(at.file.clone(), at.index))?;
            let unbacked = measured.is_none_or(|m| !backed(at, m));
            unbacked.then(|| edge(at, canon, &log.ops))
        });
        Ok(out.collect())
    }

    /// Rules the change adds that no measured module edge backs (the canvas draws them
    /// dashed). Empty when the preview or the cache does not load.
    pub fn planned_edges(&self) -> Result<Vec<PlannedEdge>> {
        let preview = self.preview(true)?;
        let Ok(l) = preview.check() else {
            return Ok(Vec::new());
        };
        if preview.cache_mismatch(&l).is_some() {
            return Ok(Vec::new());
        }
        self.added_edges(&l.cfg, Some(&lifted(&l)))
    }

    /// Rules the change adds without a `reason`, once each (a preset's rule is not repeated
    /// per level). Empty when the preview does not load.
    pub fn reasonless_rules(&self) -> Result<Vec<PlannedEdge>> {
        let Ok(cfg) = self.preview(true)?.load_config() else {
            return Ok(Vec::new());
        };
        let mut seen = BTreeSet::new();
        let mut out = self.added_edges(&cfg, None)?;
        out.retain(|e| e.reason.is_none() && seen.insert((e.file.clone(), e.rule)));
        Ok(out)
    }

    /// Publish warnings: one per rule added without a reason.
    pub(crate) fn reason_warnings(&self) -> Result<Vec<String>> {
        Ok(self.reasonless_rules()?.iter().map(no_reason).collect())
    }
}

/// `rule brickie.toml#2 (a -> b, allow) has no reason: say why with set_rule`.
pub fn no_reason(e: &PlannedEdge) -> String {
    format!(
        "{}: rule {}#{} ({} -> {}, {}) has no reason; say why with set_rule",
        brickie_model::codes::CHANGE_NO_REASON,
        e.file,
        e.rule,
        e.from,
        e.to,
        e.action
    )
}

/// `rule brickie.toml#2 allows a -> b (uses, prod) but no code uses it yet`.
pub fn unbacked(e: &PlannedEdge) -> String {
    let kinds = if e.kinds.is_empty() {
        "any kind".to_string()
    } else {
        e.kinds.join("/")
    };
    format!(
        "{}: rule {}#{} allows {} -> {} ({kinds}, {}) in {} but no measured code edge uses it yet",
        brickie_model::codes::CHANGE_PLANNED_EDGE,
        e.file,
        e.rule,
        e.from,
        e.to,
        e.scope,
        e.level
    )
}
