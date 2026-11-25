//! `deps/unlinked`: a module edge is lifted to the lowest common ancestor of its leaf paths and
//! allowed there only through a valid link ([`covering_link`]); `deps/private` (see
//! `private.rs`) checks the target's visibility on the same pass.

use super::lift::{entered, hidden, lift};
use super::private::Private;
use super::{span, tags};
use crate::config::{Brick, Config, Resolved, Why, covering_link};
use crate::diag::{Finding, Note, RuleRef, Span};
use brickie_facts::{Cache, Evidence};
use brickie_model::codes::DEPS_UNLINKED;

enum DepReason {
    /// Edge between two members of the same peer group: `(group_name, loc)`.
    IntraGroup(String, String),
    /// A link covers the edge, but no rule allows it: `(loc, why)`.
    Denied(String, String),
    /// No link covers the edge.
    Unlinked,
}

struct Dep {
    /// Level name of the common ancestor (`root/web`) and its config file.
    level: String,
    file: String,
    from: Brick,
    to: Brick,
    /// Logical unit names for `from` and `to` (group name when grouped, else brick name).
    from_unit: String,
    to_unit: String,
    /// Whether `to_unit` is a group at `level` (not a composite exporting a group).
    to_is_group: bool,
    from_module: String,
    to_module: String,
    kind: String,
    scope: String,
    /// The unit inside `to` the edge enters through, for a composite target.
    entered: Option<String>,
    evidence: Vec<Evidence>,
    reason: DepReason,
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
        let entered = entered(&l, p2);
        let from_unit = l.level.unit_of(l.from);
        let to_unit = l.level.unit_of(l.to);
        let reason = if let Some(g) = l.from.group.as_deref()
            && l.to.group.as_deref() == Some(g)
        {
            let loc = l
                .level
                .group(g)
                .map(|gr| gr.loc.clone())
                .unwrap_or_else(|| l.level.file.clone());
            DepReason::IntraGroup(g.to_string(), loc)
        } else {
            match covering_link(l.level, from_unit, to_unit, entered, &e.kind, &e.scope) {
                Ok(_) => continue,
                Err(Why::Unlinked) => DepReason::Unlinked,
                Err(Why::Denied(link, why)) => DepReason::Denied(link.loc.clone(), why),
            }
        };
        out.push(Box::new(Dep {
            level: l.level.level(),
            file: l.level.file.clone(),
            from: l.from.clone(),
            to: l.to.clone(),
            from_unit: from_unit.name.to_string(),
            to_unit: to_unit.name.to_string(),
            to_is_group: to_unit.is_group,
            from_module: e.from,
            to_module: e.to,
            kind: e.kind,
            scope: e.scope,
            entered: entered.map(String::from),
            evidence: e.evidence,
            reason,
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
        Some(match &self.reason {
            DepReason::IntraGroup(g, loc) => RuleRef {
                name: format!("bricks in group '{g}' cannot depend on each other"),
                loc: Some(loc.clone()),
            },
            DepReason::Denied(loc, why) => RuleRef {
                name: format!("link denied: {why}"),
                loc: Some(loc.clone()),
            },
            DepReason::Unlinked => RuleRef {
                name: "no link covers it".into(),
                loc: Some(self.file.clone()),
            },
        })
    }
    fn help(&self) -> Option<String> {
        if matches!(self.reason, DepReason::IntraGroup(..)) {
            return Some("remove the dependency".into());
        }
        let to = match &self.entered {
            Some(c) if !self.to_is_group => format!("{}/{c}", self.to_unit),
            _ => self.to_unit.clone(),
        };
        let level = match self.level.strip_prefix("root") {
            Some("") | None => String::new(),
            Some(rest) => format!(" --in {}", rest.trim_start_matches('/')),
        };
        let add = format!(
            "brickie design link add {} {to} --kind {} --scope {}{level} --reason \"...\"",
            self.from_unit, self.kind, self.scope
        );
        Some(match &self.reason {
            _ if self.from.independent && !self.to.independent => "remove the dependency: an \
                independent brick may use only itself, other independent bricks and external crates"
                .into(),
            DepReason::Denied(..) => {
                "remove the dependency, or ask the reviewer for a [[rule]] that allows \
                the link"
                    .into()
            }
            DepReason::Unlinked => format!("remove the dependency, or declare the link: `{add}`"),
            DepReason::IntraGroup(..) => unreachable!(),
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
