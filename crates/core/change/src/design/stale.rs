//! `design/stale`: a write based on another version than the head. Nothing is written; the
//! message says what changed since the base and how to retry.

use brickie_model::codes::DESIGN_STALE;
use brickie_model::design::Version;
use std::fmt;

/// The CLI exits with 3 on this error.
#[derive(Debug)]
pub struct Stale {
    pub message: String,
    pub head: u64,
}

impl fmt::Display for Stale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Stale {}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `12s`, `4m`, `3h`, `2d`.
pub fn ago(at: u64) -> String {
    let s = now().saturating_sub(at);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m", s / 60),
        3600..86400 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86400),
    }
}

/// `Ok` when `base` is the head; otherwise the stale error.
pub fn check(entries: &[Version], base: u64) -> Result<(), Stale> {
    let Some(head) = entries.last() else {
        return Ok(());
    };
    if base == head.v {
        return Ok(());
    }
    let at = format!(
        "error[{DESIGN_STALE}]: design is at v{} (by {}, {} ago)",
        head.v,
        head.by.name(),
        ago(head.at)
    );
    let retry = format!(
        "Re-read with `brickie design show`, then retry with --base {}.",
        head.v
    );
    let known = entries.first().is_some_and(|f| f.v <= base) && base < head.v;
    let message = if known {
        let mut lines = format!(
            "{at}, your edit was based on v{base}. Nothing was written.\nChanges since v{base}:\n"
        );
        for e in entries.iter().filter(|e| e.v > base) {
            let note = e
                .note
                .as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default();
            lines.push_str(&format!("  v{} by {}{note}:\n", e.v, e.by.name()));
            for d in &e.diff {
                lines.push_str(&format!("    {d}\n"));
            }
        }
        format!("{lines}{retry}")
    } else {
        format!("{at}; base v{base} is unknown. Nothing was written.\n{retry}")
    };
    Err(Stale {
        message,
        head: head.v,
    })
}
