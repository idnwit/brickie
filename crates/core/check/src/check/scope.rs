//! `check --brick` and `--skip`: which findings are shown. Both only hide; the whole workspace
//! is always evaluated and review decisions are applied first.

use super::{Baseline, Loaded};
use crate::config::{Config, level_name};
use crate::diag::{Diagnostic, Level, Note, finding_id};
use anyhow::{Result, bail};
use brickie_model::codes;
use std::path::Path;

/// Unaccepted findings by level.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
}

/// The options of a scoped check.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Level paths of the scope (`root/web/api`); empty means the whole workspace.
    bricks: Vec<String>,
    skip: Vec<String>,
    /// `--new`: the saved baseline and the rule files that changed since.
    gate: Option<Gate>,
}

#[derive(Debug, Clone)]
struct Gate {
    base: Baseline,
    changed: Vec<String>,
}

/// What a baseline says about one finding.
enum Verdict {
    Keep,
    Hide,
    Worse(Note),
}

impl Gate {
    fn verdict(&self, d: &Diagnostic, weight: u64) -> Verdict {
        if d.code.starts_with("stale/") {
            return Verdict::Keep;
        }
        match self.base.findings.get(&d.id) {
            None => Verdict::Keep,
            Some(b) if weight <= b.weight => Verdict::Hide,
            Some(b) => Verdict::Worse(Note {
                text: format!("worse than the baseline: {} -> {weight}", b.weight),
                span: None,
            }),
        }
    }

    /// The error that tells the rule files changed since the baseline was saved.
    fn rules_changed(&self) -> Option<Diagnostic> {
        if self.changed.is_empty() {
            return None;
        }
        let code = codes::BASELINE_RULES_CHANGED;
        Some(Diagnostic {
            id: finding_id(code, "rule files"),
            level: Level::Error,
            code: code.into(),
            message: format!(
                "rule files changed since the baseline: {}",
                self.changed.join(", ")
            ),
            span: None,
            spans: Vec::new(),
            rule: None,
            help: Some(
                "a human reviews the change (`brickie design log`, `git diff`) and re-saves the \
                 baseline with `brickie check --save-baseline`; an agent never re-saves it"
                    .into(),
            ),
            notes: Vec::new(),
            bricks: Vec::new(),
            brick_level: None,
            edges: Vec::new(),
            accepted: false,
            accepted_reason: None,
        })
    }
}

/// What a [`Filter`] leaves.
#[derive(Debug, Default)]
pub struct Filtered {
    pub shown: Vec<Diagnostic>,
    /// Hidden because no owner is in scope (`--brick`).
    pub outside: Counts,
    /// Warnings hidden by `--skip`.
    pub skipped: usize,
    /// Hidden because the baseline holds them (`--new`).
    pub baselined: Counts,
}

impl Filter {
    /// `bricks` are paths from the root (`web/api` or `root/web/api`) of existing bricks;
    /// `skip` entries may be comma separated and must be known codes.
    pub fn new(cfg: &Config, bricks: &[String], skip: &[String]) -> Result<Filter> {
        let mut paths = Vec::new();
        for b in bricks {
            let names: Vec<String> = (b.split('/'))
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let names = match names.split_first() {
                Some((root, rest)) if root == "root" => rest.to_vec(),
                _ => names,
            };
            if names.is_empty() || cfg.brick_at(&names).is_none() {
                bail!("--brick {b}: no such brick (give the path from the root, like `web/api`)");
            }
            paths.push(level_name(&names));
        }
        let known = codes::all();
        let mut codes = Vec::new();
        for c in skip.iter().flat_map(|s| s.split(',')).map(str::trim) {
            if !known.iter().any(|k| k == c) {
                bail!("--skip {c}: unknown code");
            }
            codes.push(c.to_string());
        }
        Ok(Filter {
            bricks: paths,
            skip: codes,
            gate: None,
        })
    }

    /// `--new`: hide the findings of `base` that did not get worse, and report a change of the
    /// rule files since it was saved (compared against the files under `config`).
    pub fn with_baseline(mut self, config: &Path, base: Baseline) -> Result<Filter> {
        let changed = base.changed_rule_files(config)?;
        self.gate = Some(Gate { base, changed });
        Ok(self)
    }

    pub fn gating(&self) -> bool {
        self.gate.is_some()
    }

    pub fn scoped(&self) -> bool {
        !self.bricks.is_empty()
    }

    pub fn skipping(&self) -> bool {
        !self.skip.is_empty()
    }

    pub fn skip_codes(&self) -> &[String] {
        &self.skip
    }

    fn in_scope(&self, owners: &[String]) -> bool {
        owners.is_empty()
            || owners.iter().any(|o| {
                (self.bricks.iter()).any(|b| {
                    o == b
                        || o.strip_prefix(b.as_str())
                            .is_some_and(|r| r.starts_with('/'))
                })
            })
    }

    /// Applies the scope, then the skip list, to the (review-applied) diagnostics of `l`.
    pub fn apply(&self, l: &Loaded) -> Filtered {
        let mut out = Filtered::default();
        out.shown
            .extend(self.gate.iter().filter_map(Gate::rules_changed));
        for d in &l.diagnostics {
            let mut item = d.clone();
            if let Some(g) = &self.gate {
                match g.verdict(d, l.weights.get(&d.id).copied().unwrap_or(1)) {
                    Verdict::Keep => {}
                    Verdict::Hide => {
                        count(&mut out.baselined, d);
                        continue;
                    }
                    Verdict::Worse(n) => item.notes.push(n),
                }
            }
            if self.scoped() {
                let owners = l.owners.get(&d.id).map_or(&[][..], Vec::as_slice);
                if !self.in_scope(owners) {
                    count(&mut out.outside, d);
                    continue;
                }
            }
            if d.level == Level::Warn && !d.accepted && self.skip.contains(&d.code) {
                out.skipped += 1;
                continue;
            }
            out.shown.push(item);
        }
        out
    }
}

/// Adds an unaccepted finding to `c`.
fn count(c: &mut Counts, d: &Diagnostic) {
    match (d.accepted, d.level) {
        (true, _) => {}
        (false, Level::Error) => c.errors += 1,
        (false, Level::Warn) => c.warnings += 1,
    }
}

impl Filtered {
    /// The footer lines that tell what was hidden.
    pub fn footer(&self, f: &Filter) -> String {
        let mut s = String::new();
        if f.scoped() {
            let c = self.outside;
            s += &format!(
                "outside scope: {} errors, {} warnings\n",
                c.errors, c.warnings
            );
        }
        if f.gating() {
            let c = self.baselined;
            s += &format!(
                "baseline: {} errors, {} warnings hidden\n",
                c.errors, c.warnings
            );
        }
        if f.skipping() {
            s += &format!(
                "skipped: {} warnings ({})\n",
                self.skipped,
                f.skip.join(", ")
            );
        }
        s
    }
}
