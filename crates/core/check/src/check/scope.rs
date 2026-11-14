//! `check --brick` and `--skip`: which findings are shown. Both only hide; the whole workspace
//! is always evaluated and review decisions are applied first.

use super::Loaded;
use crate::config::{Config, level_name};
use crate::diag::{Diagnostic, Level};
use anyhow::{Result, bail};
use brickie_model::codes;

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
}

/// What a [`Filter`] leaves.
#[derive(Debug, Default)]
pub struct Filtered {
    pub shown: Vec<Diagnostic>,
    /// Hidden because no owner is in scope (`--brick`).
    pub outside: Counts,
    /// Warnings hidden by `--skip`.
    pub skipped: usize,
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
        })
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
        for d in &l.diagnostics {
            if self.scoped() {
                let owners = l.owners.get(&d.id).map_or(&[][..], Vec::as_slice);
                if !self.in_scope(owners) {
                    match (d.accepted, d.level) {
                        (true, _) => {}
                        (false, Level::Error) => out.outside.errors += 1,
                        (false, Level::Warn) => out.outside.warnings += 1,
                    }
                    continue;
                }
            }
            if d.level == Level::Warn && !d.accepted && self.skip.contains(&d.code) {
                out.skipped += 1;
                continue;
            }
            out.shown.push(d.clone());
        }
        out
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
