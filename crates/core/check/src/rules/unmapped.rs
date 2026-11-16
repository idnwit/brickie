//! `bricks/unmapped`: modules that belong to no brick.

use crate::diag::{Finding, RuleRef, Span};
use brickie_model::codes::BRICKS_UNMAPPED;

pub(super) struct Unmapped {
    pub(super) module: String,
    pub(super) file: String,
    pub(super) line: u32,
}

impl Finding for Unmapped {
    fn code(&self) -> String {
        BRICKS_UNMAPPED.into()
    }
    fn subject(&self) -> String {
        self.module.clone()
    }
    fn message(&self) -> String {
        format!("module `{}` is not in any brick", self.module)
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
        Some("the module is unchecked: a human assigns it to a brick (an agent adds it to `members` only when the brick is obvious, via `brickie design brick set`)".into())
    }
}
