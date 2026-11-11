//! `deps/deny` and `deps/default`: dependency rules between bricks.

use super::codes::{DEPS_DEFAULT, DEPS_DENY};
use super::{brick_of, span, tags};
use crate::cache::{Cache, Evidence};
use crate::config::{Action, Brick, Config, DepRule, Level};
use crate::diag::{Finding, Note, RuleRef, Span};

struct Dep {
    from: Brick,
    to: Brick,
    from_module: String,
    to_module: String,
    kind: String,
    scope: String,
    evidence: Vec<Evidence>,
    /// Index and rule that denied the edge; `None` when the default applied.
    rule: Option<(usize, DepRule)>,
}

/// The most specific matching rule (brick 3 > tag 2 > `*` 1, summed over from and to); on a
/// tie a deny rule wins.
fn pick_rule<'a>(
    rules: &'a [DepRule],
    from: &Brick,
    to: &Brick,
    kind: &str,
    scope: &str,
) -> Option<(usize, &'a DepRule)> {
    let mut best: Option<(u32, usize, &DepRule)> = None;
    for (i, r) in rules.iter().enumerate() {
        if !(r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind)) || !r.scope.matches(scope) {
            continue;
        }
        let (Some(a), Some(b)) = (r.from.specificity(from), r.to.specificity(to)) else {
            continue;
        };
        let rank = a + b;
        let better = match best {
            None => true,
            Some((br, _, bp)) => {
                rank > br || (rank == br && r.action == Action::Deny && bp.action == Action::Allow)
            }
        };
        if better {
            best = Some((rank, i, r));
        }
    }
    best.map(|(_, i, r)| (i, r))
}

pub(super) fn deps(cache: &Cache, cfg: &Config, out: &mut Vec<Box<dyn Finding>>) {
    for e in cache.edges() {
        let (Some(fb), Some(tb)) = (brick_of(cache, cfg, &e.from), brick_of(cache, cfg, &e.to))
        else {
            continue;
        };
        if fb.name == tb.name {
            continue;
        }
        let rule = pick_rule(&cfg.deps.rules, fb, tb, &e.kind, &e.scope);
        let action = rule.map_or(cfg.deps.default, |(_, r)| r.action);
        if action == Action::Allow {
            continue;
        }
        out.push(Box::new(Dep {
            from: fb.clone(),
            to: tb.clone(),
            from_module: e.from,
            to_module: e.to,
            kind: e.kind,
            scope: e.scope,
            evidence: e.evidence,
            rule: rule.map(|(i, r)| (i, r.clone())),
        }));
    }
}

/// Identity of a dependency finding (the webview recomputes it to mark violating edges).
pub fn dep_subject(from: &str, to: &str, kind: &str, scope: &str) -> String {
    format!("{from}->{to}:{kind}:{scope}")
}

impl Finding for Dep {
    fn code(&self) -> String {
        if self.rule.is_some() {
            DEPS_DENY
        } else {
            DEPS_DEFAULT
        }
        .into()
    }
    fn level(&self) -> Level {
        Level::Error
    }
    fn subject(&self) -> String {
        dep_subject(&self.from_module, &self.to_module, &self.kind, &self.scope)
    }
    fn message(&self) -> String {
        let verb = match self.kind.as_str() {
            "implements" => "implements a trait of",
            "reexports" => "re-exports from",
            _ => "uses",
        };
        let in_tests = if self.scope == "test" {
            " in tests"
        } else {
            ""
        };
        format!(
            "`{}`{} {verb} `{}`{}{in_tests}",
            self.from.name,
            tags(&self.from),
            self.to.name,
            tags(&self.to)
        )
    }
    fn spans(&self) -> Vec<Span> {
        self.evidence
            .iter()
            .map(|e| {
                let label = format!(
                    "written `{}` \u{2192} defined in `{}`",
                    e.written, self.to_module
                );
                span(e, Some(label))
            })
            .collect()
    }
    fn rule(&self) -> Option<RuleRef> {
        Some(match &self.rule {
            Some((i, r)) => RuleRef {
                name: match &r.reason {
                    Some(why) => format!("deps.rule[{i}] \"{why}\""),
                    None => format!("deps.rule[{i}]"),
                },
                loc: Some(r.loc.clone()),
            },
            None => RuleRef {
                name: "deps.default = \"deny\"".into(),
                loc: None,
            },
        })
    }
    fn help(&self) -> Option<String> {
        Some(match &self.rule {
            Some((_, r)) if r.help.is_some() => r.help.clone().unwrap_or_default(),
            Some(_) => "remove the dependency, or change the rule if it is intended".into(),
            None => "remove the dependency, or add a [[deps.rule]] that allows it".into(),
        })
    }
    fn bricks(&self) -> Vec<String> {
        vec![self.from.name.clone(), self.to.name.clone()]
    }
    fn notes(&self) -> Vec<Note> {
        vec![Note {
            text: format!(
                "modules: {} -> {} ({}, {})",
                self.from_module, self.to_module, self.kind, self.scope
            ),
            span: None,
        }]
    }
}
