//! `deps/private`: an edge into a composite reaches a child the composite does not export.

use super::deps::{at_level, dep_subject, in_tests, verb};
use super::span;
use crate::config::Level;
use crate::diag::{Finding, Note, RuleRef, Span};
use brickie_facts::Evidence;
use brickie_model::codes::DEPS_PRIVATE;

pub(super) struct Private {
    /// The composite that hides `hidden`: its level name and config file.
    pub(super) level: String,
    pub(super) file: String,
    pub(super) hidden: String,
    /// The siblings at the common ancestor.
    pub(super) from: String,
    /// Full level path of the source brick (`root/web/ui`).
    pub(super) owner: String,
    pub(super) to: String,
    pub(super) from_module: String,
    pub(super) to_module: String,
    pub(super) kind: String,
    pub(super) scope: String,
    pub(super) evidence: Vec<Evidence>,
}

impl Finding for Private {
    fn code(&self) -> String {
        DEPS_PRIVATE.into()
    }
    fn level(&self) -> Level {
        Level::Error
    }
    fn subject(&self) -> String {
        dep_subject(&self.from_module, &self.to_module, &self.kind, &self.scope)
    }
    fn message(&self) -> String {
        format!(
            "`{}` {} `{}`, which `{}` does not export{}{}",
            self.from,
            verb(&self.kind),
            self.hidden,
            self.level.rsplit('/').next().unwrap_or_default(),
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
        Some(RuleRef {
            name: "exports".into(),
            loc: Some(self.file.clone()),
        })
    }
    fn help(&self) -> Option<String> {
        Some(format!(
            "use an exported brick of `{}` instead; if `{}` is meant to be public, add it to `exports` in that file (or a scoped entry such as {{ brick = \"{}\", scope = \"test\" }}); or run `brickie design export set {} [--scope prod|test]`",
            self.to, self.hidden, self.hidden, self.hidden
        ))
    }
    /// Names are local to the finding's level (the composite): the hidden child.
    fn bricks(&self) -> Vec<String> {
        vec![self.hidden.clone()]
    }
    fn owners(&self) -> Vec<String> {
        vec![self.owner.clone()]
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
