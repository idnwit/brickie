//! Applying the review file to the findings: acceptance, its weight bound and orphans.

use super::orphan::Orphan;
use crate::diag::{self, Diagnostic, Note};
use crate::review::{Accept, Review, review_path};
use std::collections::BTreeMap;
use std::path::Path;

/// Marks the accepted findings in `diags`. An acceptance with a `weight` holds only while the
/// finding's current weight (`weights`, by id) stays within it; above it the finding stays open
/// and gets a note. `stale/*` findings are never accepted. Then appends a `review/orphan`
/// warning for every accept that matches no finding, unless the facts are stale (ids may then
/// be missing only because the cache is old).
pub fn apply(
    config: &Path,
    review: &Review,
    weights: &BTreeMap<String, u64>,
    diags: &mut Vec<Diagnostic>,
) {
    let stale = diags.iter().any(|d| d.code.starts_with("stale/"));
    for d in diags.iter_mut() {
        if d.code.starts_with("stale/") {
            continue;
        }
        if let Some(a) = review.accept.iter().find(|a| a.id == d.id) {
            accept(d, a, weights.get(&d.id).copied().unwrap_or(1));
        }
    }
    if stale {
        return;
    }
    let text = std::fs::read_to_string(review_path(config)).unwrap_or_default();
    for a in &review.accept {
        if !diags.iter().any(|d| d.id == a.id) {
            let o = Orphan::new(&a.id, "brickie.review.toml", &text);
            diags.push(diag::diagnostic(&o));
        }
    }
}

fn accept(d: &mut Diagnostic, a: &Accept, now: u64) {
    match a.weight {
        Some(w) if now > w => d.notes.push(Note {
            text: format!("worse than accepted: {w} -> {now}"),
            span: None,
        }),
        _ => {
            d.accepted = true;
            d.accepted_reason = Some(a.reason.clone());
        }
    }
}
