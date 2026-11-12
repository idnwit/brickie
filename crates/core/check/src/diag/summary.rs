//! Counting findings for the summary line.

use super::Diagnostic;
use brickie_model::Level;

pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub accepted: usize,
}

pub fn summarize(ds: &[Diagnostic]) -> Summary {
    let mut s = Summary {
        errors: 0,
        warnings: 0,
        accepted: 0,
    };
    for d in ds {
        if d.accepted {
            s.accepted += 1;
        } else if d.level == Level::Error {
            s.errors += 1;
        } else {
            s.warnings += 1;
        }
    }
    s
}
