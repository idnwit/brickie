//! `deps/independent`: an independent root brick uses a workspace module outside itself.

use super::deps::{dep_subject, in_tests, verb};
use super::span;
use crate::config::{Action, Brick, Config, Level, Sel};
use crate::diag::{Finding, RuleRef, Span};
use brickie_facts::{Edge, Evidence};
use brickie_model::codes::DEPS_INDEPENDENT;

/// The independent root brick whose leaf path is `p1`, if any.
pub(super) fn independent_source<'a>(cfg: &'a Config, p1: &[String]) -> Option<&'a Brick> {
    let [name] = p1 else { return None };
    cfg.brick(name).filter(|b| b.independent)
}

/// An independent brick may use another independent brick only when an explicit allow rule
/// (brick name or tag on both sides, never `*`) covers the edge; the normal rules then decide.
pub(super) fn independent_pair_allowed(
    cfg: &Config,
    from: &Brick,
    to: Option<&[String]>,
    kind: &str,
    scope: &str,
) -> bool {
    let Some(to) = to.and_then(|p| independent_source(cfg, p)) else {
        return false;
    };
    cfg.deps.rules.iter().any(|r| {
        r.action == Action::Allow
            && r.from != Sel::Any
            && r.to != Sel::Any
            && r.from.specificity(from).is_some()
            && r.to_rank(to, None).is_some()
            && (r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind))
            && r.scope.matches(scope)
    })
}

pub(super) struct Independent {
    from: String,
    /// The brick the target module belongs to, or the module id when it is unmapped.
    to: String,
    from_module: String,
    to_module: String,
    kind: String,
    scope: String,
    evidence: Vec<Evidence>,
}

impl Independent {
    pub(super) fn new(from: String, to: String, e: Edge) -> Self {
        Self {
            from,
            to,
            from_module: e.from,
            to_module: e.to,
            kind: e.kind,
            scope: e.scope,
            evidence: e.evidence,
        }
    }
}

impl Finding for Independent {
    fn weight(&self) -> u64 {
        (self.evidence.len() as u64).max(1)
    }
    fn code(&self) -> String {
        DEPS_INDEPENDENT.into()
    }
    fn level(&self) -> Level {
        Level::Error
    }
    fn subject(&self) -> String {
        dep_subject(&self.from_module, &self.to_module, &self.kind, &self.scope)
    }
    fn message(&self) -> String {
        format!(
            "`{}` is independent and {} `{}`{}",
            self.from,
            verb(&self.kind),
            self.to,
            in_tests(&self.scope)
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
        Some(RuleRef {
            name: "independent = true".into(),
            loc: None,
        })
    }
    fn help(&self) -> Option<String> {
        Some(
            "remove the dependency: an independent brick may use only itself and external \
             crates; move the shared code into the brick or into a published crate"
                .into(),
        )
    }
    fn bricks(&self) -> Vec<String> {
        vec![self.from.clone()]
    }
    fn owners(&self) -> Vec<String> {
        vec![format!("root/{}", self.from)]
    }
    fn brick_level(&self) -> Option<String> {
        Some("root".into())
    }
}
