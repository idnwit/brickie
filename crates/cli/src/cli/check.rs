//! `brickie check`: print the diagnostics, exit non-zero when an unaccepted error exists.
//!
//! Exit code: 0 clean, 1 unaccepted error findings, 2 config or cache problem.

use super::args::{ColorArg, Format};
use anyhow::Result;
use brickie_check::check::{self, Loaded};
use brickie_check::diag::{self, Color};
use brickie_model::Level;
use std::path::Path;

pub fn run(config: &Path, format: Format, color: ColorArg) -> i32 {
    let json = matches!(format, Format::Json);
    let color = match color {
        ColorArg::Auto => Color::Auto,
        ColorArg::Always => Color::Always,
        ColorArg::Never => Color::Never,
    };
    match check(config, json, color) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    }
}

fn check(config: &Path, json: bool, color: Color) -> Result<i32> {
    let Loaded {
        cache,
        diagnostics: ds,
        ..
    } = check::load(config)?;
    let root = Path::new(&cache.workspace_root);

    let s = diag::summarize(&ds);
    if json {
        let out = serde_json::json!({
            "summary": {"errors": s.errors, "warnings": s.warnings, "accepted": s.accepted},
            "diagnostics": ds,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print!("{}", diag::render_all(&ds, root, color.enabled()));
    }
    Ok(i32::from(
        ds.iter().any(|d| d.level == Level::Error && !d.accepted),
    ))
}
