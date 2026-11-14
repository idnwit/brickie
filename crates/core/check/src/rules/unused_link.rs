//! `deps/unused`: an allow rule between two fully measured bricks that no edge uses.

use super::links::{Link, LinkState};
use crate::config::{Level, Scope, level_name};
use crate::diag::{Finding, RuleRef, Span};
use brickie_model::codes::DEPS_UNUSED;

pub(super) struct UnusedLink(pub(super) Link);

/// ` no `uses` edge in test scope`: what the rule looks for.
pub fn filter_text(l: &Link) -> String {
    let kinds = if l.kinds.is_empty() {
        "edge".to_string()
    } else {
        let k: Vec<String> = l.kinds.iter().map(|k| format!("`{k}`")).collect();
        format!("{} edge", k.join(" or "))
    };
    let scope = match l.scope {
        Scope::All => "any scope".to_string(),
        s => format!("{} scope", s.name()),
    };
    format!("no {kinds} in {scope}")
}

impl Finding for UnusedLink {
    fn code(&self) -> String {
        DEPS_UNUSED.into()
    }
    fn level(&self) -> Level {
        Level::Warn
    }
    /// Level, ends, kinds and scope: no file line, so the id survives edits of the toml.
    fn subject(&self) -> String {
        let l = &self.0;
        format!(
            "{}:{}->{}:{}:{}",
            level_name(&l.level),
            l.from,
            l.to,
            l.kinds.join(","),
            l.scope.name()
        )
    }
    fn message(&self) -> String {
        let l = &self.0;
        format!(
            "rule `{}` -> `{}` is unused: {} (in {})",
            l.from,
            l.to,
            filter_text(l),
            level_name(&l.level)
        )
    }
    fn spans(&self) -> Vec<Span> {
        let (file, line) = match self.0.loc.rsplit_once(':') {
            Some((f, n)) => (f.to_string(), n.parse().unwrap_or(0)),
            None => (self.0.file.clone(), 0),
        };
        vec![Span {
            file,
            line,
            col: None,
            len: None,
            label: Some("the rule".into()),
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        Some(RuleRef {
            name: "deps.rule (allow)".into(),
            loc: Some(self.0.loc.clone()),
        })
    }
    fn help(&self) -> Option<String> {
        Some(
            "write the code that uses it, fix the rule's kinds or scope, or remove the rule \
             (`brickie design rule rm`)"
                .into(),
        )
    }
    fn bricks(&self) -> Vec<String> {
        vec![self.0.from.clone(), self.0.to.clone()]
    }
    fn brick_level(&self) -> Option<String> {
        Some(level_name(&self.0.level))
    }
}

/// One `deps/unused` finding per unused link.
pub(super) fn unused_links(links: &[Link]) -> Vec<Box<dyn Finding>> {
    (links.iter().filter(|l| l.state == LinkState::Unused))
        .map(|l| Box::new(UnusedLink(l.clone())) as Box<dyn Finding>)
        .collect()
}
