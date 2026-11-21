//! `config/stale-allow`: a `// brickie: allow(..) -- reason` directive that has no reason, names
//! a code that cannot be exempted, or exempts an item that is not unused anyway, so exemptions
//! cannot rot.

use crate::config::{Resolved, level_name};
use crate::diag::{Finding, RuleRef, Span};
use brickie_facts::{Allow, Cache, Item};
use brickie_model::codes::{CONFIG_STALE_ALLOW, INTERFACE_UNUSED};
use std::collections::HashSet;

/// Codes a directive can exempt: the per-item findings.
const ALLOWABLE: [&str; 1] = [INTERFACE_UNUSED];

struct Stale {
    file: String,
    /// Line of the directive comment.
    line: u32,
    brick: Option<Vec<String>>,
    problems: Vec<String>,
}

impl Finding for Stale {
    fn code(&self) -> String {
        CONFIG_STALE_ALLOW.into()
    }
    fn subject(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
    fn message(&self) -> String {
        format!("allow directive is stale: {}", self.problems.join("; "))
    }
    fn spans(&self) -> Vec<Span> {
        vec![Span {
            file: self.file.clone(),
            line: self.line,
            col: None,
            len: None,
            label: None,
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        Some(
            "remove the directive, or fix it to `// brickie: allow(interface/unused) -- <reason>`"
                .into(),
        )
    }
    fn bricks(&self) -> Vec<String> {
        self.brick
            .iter()
            .filter_map(|b| b.last().cloned())
            .collect()
    }
    fn brick_level(&self) -> Option<String> {
        let b = self.brick.as_ref()?;
        Some(level_name(&b[..b.len().saturating_sub(1)]))
    }
}

/// Why the directive of `item` is stale; empty when it is fine.
fn problems(item: &Item, a: &Allow, exempt: &HashSet<(&str, &str)>) -> Vec<String> {
    let mut out = Vec::new();
    if a.reason.is_empty() {
        out.push("it has no reason (add ` -- <why>`)".to_string());
    }
    if a.codes.is_empty() {
        out.push("it lists no code".into());
    }
    for c in a.codes.iter().filter(|c| !ALLOWABLE.contains(&c.as_str())) {
        out.push(format!("`{c}` cannot be exempted"));
    }
    let listed = a.codes.iter().any(|c| c == INTERFACE_UNUSED);
    let exempted = exempt.contains(&(item.module.as_str(), item.symbol.as_str()));
    if listed && !a.reason.is_empty() && !exempted {
        out.push(format!(
            "`{}` is not unused anyway (it is used, published, or not an interface item)",
            item.name
        ));
    }
    out
}

/// Stale directives of every recorded item. `exempt` holds the `(module, symbol)` of items an
/// `allow(interface/unused)` directive actually exempts (they would be reported otherwise).
pub(super) fn stale_allows(
    cache: &Cache,
    res: &Resolved,
    exempt: &HashSet<(&str, &str)>,
) -> Vec<Box<dyn Finding>> {
    let mut out: Vec<Box<dyn Finding>> = Vec::new();
    for (file, fd) in &cache.files {
        for item in &fd.items {
            let Some(a) = &item.allow else { continue };
            let problems = problems(item, a, exempt);
            if problems.is_empty() {
                continue;
            }
            out.push(Box::new(Stale {
                file: file.clone(),
                line: a.line,
                brick: res.path(&item.module).map(<[String]>::to_vec),
                problems,
            }));
        }
    }
    out
}
