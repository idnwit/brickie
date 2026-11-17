//! The design's own findings and todo: config issues (`config/missing` and the partition
//! errors) and denied links are findings; the planned work is the todo of `check::Todo`.

use crate::config::{Issue, LinkState, level_name};
use crate::diag::{Finding, RuleRef, Span};
use brickie_model::codes::DESIGN_LINK_DENIED;

/// A config issue as a finding; owned by the brick level it happened in.
pub struct IssueFinding(pub Issue);

impl Finding for IssueFinding {
    fn code(&self) -> String {
        self.0.code.into()
    }
    fn subject(&self) -> String {
        let i = &self.0;
        format!(
            "{}\0{}\0{}",
            i.message,
            i.module.as_deref().unwrap_or(""),
            i.member.as_deref().unwrap_or("")
        )
    }
    fn message(&self) -> String {
        let m = &self.0.message;
        let code = self.0.code;
        m.strip_prefix(code)
            .and_then(|r| r.strip_prefix(": "))
            .unwrap_or(m)
            .to_string()
    }
    fn spans(&self) -> Vec<Span> {
        vec![Span {
            file: self.0.file.clone(),
            line: 1,
            col: None,
            len: None,
            label: None,
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        None
    }
    fn brick_level(&self) -> Option<String> {
        Some(level_name(&self.0.level))
    }
    fn owners(&self) -> Vec<String> {
        vec![level_name(&self.0.level)]
    }
}

/// `a -> b` in level `root/web`.
pub fn named(s: &LinkState) -> String {
    format!(
        "link {} -> {} in {}",
        s.link.from,
        s.link.to,
        level_name(&s.level)
    )
}

/// The message of a denied link.
pub fn denied_text(s: &LinkState, why: &str) -> String {
    format!(
        "{DESIGN_LINK_DENIED}: {} is not allowed: {why}; add a [[rule]] that allows it \
         (`brickie design rule add`) or remove the link",
        named(s)
    )
}

/// A link no rule of its level allows.
pub struct LinkDenied(pub LinkState, pub String);

impl Finding for LinkDenied {
    fn code(&self) -> String {
        DESIGN_LINK_DENIED.into()
    }
    fn subject(&self) -> String {
        self.0.link.loc.clone()
    }
    fn message(&self) -> String {
        let t = denied_text(&self.0, &self.1);
        t.strip_prefix("design/link-denied: ").unwrap_or(&t).into()
    }
    fn spans(&self) -> Vec<Span> {
        let loc = &self.0.link.loc;
        let (file, line) = match loc.rsplit_once(':') {
            Some((f, n)) => (f.to_string(), n.parse().unwrap_or(1)),
            None => (self.0.file.clone(), 1),
        };
        vec![Span {
            file,
            line,
            col: None,
            len: None,
            label: None,
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        None
    }
    fn brick_level(&self) -> Option<String> {
        Some(level_name(&self.0.level))
    }
    fn owners(&self) -> Vec<String> {
        vec![level_name(&self.0.level)]
    }
}
