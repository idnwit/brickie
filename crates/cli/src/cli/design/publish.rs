//! `design check`, `publish`, `rebase` and `discard`: the draft's way into the published tree.

use super::render;
use anyhow::Result;
use brickie_change::{Blocked, By, Design, Gate};
use std::fmt::Write;

/// The text of `design check`: what blocks a publish, and the findings that do not.
pub fn checklist(head: &str, g: &Gate) -> String {
    let mut s = format!("design check: {head}\n");
    match g.ok() {
        true => s.push_str("ready: the code matches the design's structure\n"),
        false => {
            let _ = writeln!(s, "blocked: {} item(s) to do in the code", g.blocking.len());
            for b in &g.blocking {
                let at = [b.file.as_deref(), b.member.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ");
                let at = match at.is_empty() {
                    true => String::new(),
                    false => format!("  [{at}]"),
                };
                let _ = writeln!(s, "  - {}{at}", b.message);
            }
        }
    }
    if !g.info.is_empty() {
        let _ = writeln!(s, "not blocking: {} finding(s)", g.info.len());
        for i in &g.info {
            let _ = writeln!(s, "  - {i}");
        }
    }
    s
}

/// Compare the code with the working design; blocked (exit 4) exactly when publish would be.
pub fn check(d: &Design, json: bool) -> Result<()> {
    let head = d.head()?;
    let g = d.check_working()?;
    let value = serde_json::json!({
        "v": head.v, "track": head.track, "forked_from": head.forked_from,
        "ready": g.ok(), "blocking": g.blocking, "info": g.info,
    });
    let name = render::track(head.track, head.v, head.forked_from);
    super::print(json, &value, || checklist(&name, &g))?;
    match g.ok() {
        true => Ok(()),
        false => Err(Blocked { gate: g }.into()),
    }
}

pub fn publish(d: &Design, json: bool, base: u64, note: Option<String>) -> Result<()> {
    let out = d.publish(base, By::Agent, note)?;
    super::print(json, &render::outcome_json(&out), || render::outcome(&out))
}
