//! The three output formats of `brickie check`.

use anyhow::Result;
use brickie_check::check::{Filter, Filtered, Loaded};
use brickie_check::diag::{self, Color};
use std::path::Path;

pub fn json(filter: &Filter, found: &Filtered) -> Result<()> {
    let ds = &found.shown;
    let mut out = serde_json::json!({
        "summary": {"errors": ds.len()},
        "diagnostics": ds,
    });
    if filter.scoped() {
        out["outside_scope"] = serde_json::json!({"errors": found.outside});
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub fn text(l: &Loaded, filter: &Filter, found: &Filtered, color: Color) {
    let root = Path::new(&l.cache.workspace_root);
    print!("{}", diag::render_all(&found.shown, root, color.enabled()));
    print!("{}", found.footer(filter));
}

pub fn short(l: &Loaded, filter: &Filter, found: &Filtered) {
    let root = Path::new(&l.cache.workspace_root);
    print!("{}", diag::render_short(&found.shown, root));
    print!("{}", found.footer(filter));
}
