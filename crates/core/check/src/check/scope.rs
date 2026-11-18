//! `check --brick`: which findings are shown. It only hides; the whole workspace is always
//! evaluated.

use super::Loaded;
use crate::config::{Config, level_name};
use crate::diag::Diagnostic;
use anyhow::{Result, bail};

/// The options of a scoped check.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Level paths of the scope (`root/web/api`); empty means the whole workspace.
    bricks: Vec<String>,
}

/// What a [`Filter`] leaves.
#[derive(Debug, Default)]
pub struct Filtered {
    /// The findings in scope.
    pub shown: Vec<Diagnostic>,
    /// Hidden because no owner is in scope.
    pub outside: usize,
}

impl Filter {
    /// `bricks` are paths from the root (`web/api` or `root/web/api`) of existing bricks.
    pub fn new(cfg: &Config, bricks: &[String]) -> Result<Filter> {
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
        Ok(Filter { bricks: paths })
    }

    /// Whether any brick narrows the scope.
    pub fn scoped(&self) -> bool {
        !self.bricks.is_empty()
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

    /// Applies the scope to the diagnostics of `l`.
    pub fn apply(&self, l: &Loaded) -> Filtered {
        let mut out = Filtered::default();
        for d in &l.diagnostics {
            if self.scoped() {
                let owners = l.owners.get(&d.id).map_or(&[][..], Vec::as_slice);
                if !self.in_scope(owners) {
                    out.outside += 1;
                    continue;
                }
            }
            out.shown.push(d.clone());
        }
        out
    }
}

impl Filtered {
    /// The footer line that tells what was hidden.
    pub fn footer(&self, f: &Filter) -> String {
        if f.scoped() {
            format!("outside scope: {} errors\n", self.outside)
        } else {
            String::new()
        }
    }
}
