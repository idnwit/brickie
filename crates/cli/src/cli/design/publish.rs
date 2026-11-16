//! `design publish`, `rebase` and `discard`: the draft's way into the published tree.

use super::render;
use anyhow::Result;
use brickie_change::{By, Design, Gate};
use std::fmt::Write;

/// The checklist of `publish --dry-run`: what blocks, and the dependency findings that do not.
pub fn checklist(base: u64, published: u64, g: &Gate) -> String {
    let mut s = format!(
        "publish dry run: {} (nothing written)\n",
        render::track(brickie_change::Track::Draft, base, Some(published))
    );
    match g.ok() {
        true => s.push_str("ready: the code matches the draft's structure\n"),
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
        let _ = writeln!(s, "not blocking: {} dependency finding(s)", g.info.len());
        for i in &g.info {
            let _ = writeln!(s, "  - {i}");
        }
    }
    s
}

pub fn publish(
    d: &Design,
    json: bool,
    base: u64,
    dry_run: bool,
    note: Option<String>,
) -> Result<()> {
    if dry_run {
        let g = d.publish_check(base)?;
        let published = d.published_head()?.v;
        let value = serde_json::json!({
            "dry_run": true, "draft": base, "forked_from": published,
            "ready": g.ok(), "blocking": g.blocking, "info": g.info,
        });
        return super::print(json, &value, || checklist(base, published, &g));
    }
    let out = d.publish(base, By::Agent, note)?;
    super::print(json, &render::outcome_json(&out), || render::outcome(&out))
}
