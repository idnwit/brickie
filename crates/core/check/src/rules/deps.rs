//! `deps/unlinked`: a module edge is lifted to the lowest common ancestor of its leaf paths and
//! allowed there only through a valid link ([`covering_link`]); `deps/private` (see
//! `private.rs`) checks the target's visibility on the same pass.

use super::lift::{hidden, lift};
use super::private::Private;
use super::{span, tags};
use crate::config::{Brick, Config, Resolved, Why, covering_link};
use crate::diag::{Finding, Note, RuleRef, Span};
use brickie_facts::{Cache, Evidence};
use brickie_model::codes::DEPS_UNLINKED;

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
    /// The child of `to` the edge enters through, for a composite target.
    entered: Option<String>,
    evidence: Vec<Evidence>,
    /// The link that covers the edge but that no rule allows (`loc`, why), if any.
    denied: Option<(String, String)>,
}

pub(super) fn deps(cache: &Cache, cfg: &Config, res: &Resolved, out: &mut Vec<Box<dyn Finding>>) {
    for e in cache.edges() {
        let (Some(p1), Some(p2)) = (res.path(&e.from), res.path(&e.to)) else {
            continue;
        };
        let Some(l) = lift(cfg, p1, p2) else { continue };
        if let Some((composite, child)) = hidden(cfg, p2, l.k) {
            out.push(Box::new(Private {
                level: composite.level(),
                file: composite.file.clone(),
                hidden: child.to_string(),
                from: l.from.name.clone(),
                owner: format!("{}/{}", l.level.level(), l.from.name),
                to: l.to.name.clone(),
                from_module: e.from.clone(),
                to_module: e.to.clone(),
                kind: e.kind.clone(),
                scope: e.scope.clone(),
                evidence: e.evidence.clone(),
            }));
        }
        let entered = p2.get(l.k + 1).map(String::as_str);
        let denied = match covering_link(l.level, l.from, l.to, entered, &e.kind, &e.scope) {
            Ok(_) => continue,
            Err(Why::Unlinked) => None,
            Err(Why::Denied(link, why)) => Some((link.loc.clone(), why)),
        };
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
            denied,
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
        DEPS_UNLINKED.into()
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
        Some(match &self.denied {
            Some((loc, why)) => RuleRef {
                name: format!("link denied: {why}"),
                loc: Some(loc.clone()),
            },
            None => RuleRef {
                name: "no link covers it".into(),
                loc: Some(self.file.clone()),
            },
        })
    }
    fn help(&self) -> Option<String> {
        let to = match &self.entered {
            Some(c) => format!("{}/{c}", self.to.name),
            None => self.to.name.clone(),
        };
        let level = match self.level.strip_prefix("root") {
            Some("") | None => String::new(),
            Some(rest) => format!(" --in {}", rest.trim_start_matches('/')),
        };
        let add = format!(
            "brickie design link add {} {to} --kind {} --scope {}{level} --reason \"...\"",
            self.from.name, self.kind, self.scope
        );
        Some(match &self.denied {
            _ if self.from.independent && !self.to.independent => "remove the dependency: an \
                independent brick may use only itself, other independent bricks and external crates"
                .into(),
            Some(_) => "remove the dependency, or ask the reviewer for a [[rule]] that allows \
                the link"
                .into(),
            None => format!("remove the dependency, or declare the link: `{add}`"),
        })
    }
    fn bricks(&self) -> Vec<String> {
        vec![self.from.name.clone(), self.to.name.clone()]
    }
    fn owners(&self) -> Vec<String> {
        vec![format!("{}/{}", self.level, self.from.name)]
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
