//! `deps/deny` and `deps/default`: dependency rules between sibling bricks. A module edge is
//! lifted to the lowest common ancestor of its leaf paths and checked against that level's
//! rules; `deps/private` (see `private.rs`) checks the target's visibility on the same pass.

use super::lift::{hidden, lift};
use super::private::Private;
use super::{span, tags};
use crate::config::{Action, Brick, Config, DepRule, Level, Resolved};
use crate::diag::{Finding, Note, RuleRef, Span};
use brickie_facts::{Cache, Evidence};
use brickie_model::codes::{DEPS_DEFAULT, DEPS_DENY};

struct Dep {
    /// Level name of the common ancestor (`root/web`) and its config file.
    level: String,
    file: String,
    from: Brick,
    to: Brick,
    from_module: String,
    to_module: String,
    kind: String,
    scope: String,
    /// The child of `to` the edge enters through; `None` when `to` is a leaf.
    entered: Option<String>,
    evidence: Vec<Evidence>,
    /// Index and rule that denied the edge; `None` when the default applied.
    rule: Option<(usize, DepRule)>,
}

/// The most specific matching rule (brick 3 > tag 2 > `*` 1, summed over from and to); on a
/// tie a deny rule wins. A rule with `via` matches only an edge entering `to` through one of
/// those children and scores one more on the `to` side.
fn pick_rule<'a>(
    rules: &'a [DepRule],
    from: &Brick,
    to: &Brick,
    entered: Option<&str>,
    kind: &str,
    scope: &str,
) -> Option<(usize, &'a DepRule)> {
    let mut best: Option<(u32, usize, &DepRule)> = None;
    for (i, r) in rules.iter().enumerate() {
        if !(r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind)) || !r.scope.matches(scope) {
            continue;
        }
        let (Some(a), Some(b)) = (r.from.specificity(from), r.to_rank(to, entered)) else {
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

pub(super) fn deps(cache: &Cache, cfg: &Config, res: &Resolved, out: &mut Vec<Box<dyn Finding>>) {
    for e in cache.edges() {
        let (Some(p1), Some(p2)) = (res.path(&e.from), res.path(&e.to)) else {
            continue;
        };
        let Some(l) = lift(cfg, p1, p2) else { continue };
        if let Some((composite, child)) = hidden(cfg, p2, l.k, &e.scope) {
            out.push(Box::new(Private {
                level: composite.level(),
                file: composite.file.clone(),
                hidden: child.to_string(),
                from: l.from.name.clone(),
                to: l.to.name.clone(),
                from_module: e.from.clone(),
                to_module: e.to.clone(),
                kind: e.kind.clone(),
                scope: e.scope.clone(),
                evidence: e.evidence.clone(),
            }));
        }
        let rules = &l.level.deps;
        let entered = p2.get(l.k + 1).map(String::as_str);
        let rule = pick_rule(&rules.rules, l.from, l.to, entered, &e.kind, &e.scope);
        if rule.map_or(rules.default, |(_, r)| r.action) == Action::Allow {
            continue;
        }
        out.push(Box::new(Dep {
            level: l.level.level(),
            file: l.level.file.clone(),
            from: l.from.clone(),
            to: l.to.clone(),
            from_module: e.from,
            to_module: e.to,
            kind: e.kind,
            scope: e.scope,
            entered: entered.map(String::from),
            evidence: e.evidence,
            rule: rule.map(|(i, r)| (i, r.clone())),
        }));
    }
}

pub(super) fn verb(kind: &str) -> &'static str {
    match kind {
        "implements" => "implements a trait of",
        "reexports" => "re-exports from",
        _ => "uses",
    }
}

pub(super) fn in_tests(scope: &str) -> &'static str {
    if scope == "test" { " in tests" } else { "" }
}

/// ` via `ports-out`` for an edge entering a composite through that child.
pub(super) fn via_note(entered: Option<&str>) -> String {
    entered.map_or_else(String::new, |e| format!(" via `{e}`"))
}

/// ` (in root/web)` below the root, empty at the root.
pub(super) fn at_level(level: &str) -> String {
    if level == "root" {
        String::new()
    } else {
        format!(" (in {level})")
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
        format!(
            "`{}`{} {} `{}`{}{}{}{}",
            self.from.name,
            tags(&self.from),
            verb(&self.kind),
            self.to.name,
            tags(&self.to),
            via_note(self.entered.as_deref()),
            in_tests(&self.scope),
            at_level(&self.level)
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
                loc: Some(self.file.clone()),
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
    fn brick_level(&self) -> Option<String> {
        Some(self.level.clone())
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
