//! `interface/unused`: a leaf brick declares an item that no code outside the brick and no test
//! uses. Reported per leaf, so an item counts as used when a sibling (or anything outside)
//! or a test references it.

use super::declared::declared;
use super::uses::Index;
use crate::config::{Config, Level, Resolved, level_name};
use crate::diag::{Finding, RuleRef, Span};
use brickie_facts::Cache;
use brickie_model::codes::INTERFACE_UNUSED;
use std::collections::BTreeSet;

struct Unused {
    leaf: Vec<String>,
    module: String,
    symbol: String,
    kind: String,
    name: String,
    file: String,
    line: u32,
}

impl Finding for Unused {
    fn code(&self) -> String {
        INTERFACE_UNUSED.into()
    }
    fn level(&self) -> Level {
        Level::Warn
    }
    fn subject(&self) -> String {
        format!("{}:{}:{}", level_name(&self.leaf), self.module, self.symbol)
    }
    fn message(&self) -> String {
        format!(
            "{} `{}` is declared by `{}` but nothing outside it uses it and no test does",
            self.kind,
            self.name,
            self.leaf.last().map(String::as_str).unwrap_or_default()
        )
    }
    fn spans(&self) -> Vec<Span> {
        vec![Span {
            file: self.file.clone(),
            line: self.line,
            col: None,
            len: None,
            label: Some(format!("defined in `{}`", self.module)),
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        Some(
            "make it private (or narrower, e.g. `pub(super)`) if only this brick needs it, \
             remove it if dead, or add a test that exercises it"
                .into(),
        )
    }
    fn bricks(&self) -> Vec<String> {
        self.leaf.last().cloned().into_iter().collect()
    }
    fn brick_level(&self) -> Option<String> {
        Some(level_name(&self.leaf[..self.leaf.len().saturating_sub(1)]))
    }
}

/// One finding per declared item of every leaf brick without a use.
pub fn unused(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<Box<dyn Finding>> {
    let idx = Index::new(cache, res);
    let leaves: BTreeSet<&Vec<String>> = res.paths.values().collect();
    let mut out: Vec<Box<dyn Finding>> = Vec::new();
    for leaf in leaves {
        for d in declared(cache, cfg, res, leaf) {
            if !idx.uses(d.item, leaf).is_empty() {
                continue;
            }
            out.push(Box::new(Unused {
                leaf: leaf.clone(),
                module: d.item.module.clone(),
                symbol: d.item.symbol.clone(),
                kind: d.item.kind.clone(),
                name: d.item.name.clone(),
                file: d.file.to_string(),
                line: d.item.line,
            }));
        }
    }
    out
}
