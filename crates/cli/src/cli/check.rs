//! `brickie check`: print the diagnostics, exit non-zero when an unaccepted error exists.
//!
//! Exit code: 0 clean, 1 unaccepted error findings, 2 config or cache problem.

use super::args::{ColorArg, Format};
use anyhow::Result;
use brickie_check::check::{self, Filter};
use brickie_check::diag::{self, Color};
use brickie_model::Level;
use std::path::Path;

pub fn run(
    config: &Path,
    format: Format,
    color: ColorArg,
    bricks: &[String],
    skip: &[String],
) -> i32 {
    let json = matches!(format, Format::Json);
    let color = match color {
        ColorArg::Auto => Color::Auto,
        ColorArg::Always => Color::Always,
        ColorArg::Never => Color::Never,
    };
    match check(config, json, color, bricks, skip) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    }
}

fn check(
    config: &Path,
    json: bool,
    color: Color,
    bricks: &[String],
    skip: &[String],
) -> Result<i32> {
    let loaded = check::load(config)?;
    let filter = Filter::new(&loaded.cfg, bricks, skip)?;
    let found = filter.apply(&loaded);
    let (ds, outside, skipped) = (&found.shown, found.outside, found.skipped);
    let root = Path::new(&loaded.cache.workspace_root);

    let s = diag::summarize(ds);
    if json {
        let mut out = serde_json::json!({
            "summary": {"errors": s.errors, "warnings": s.warnings, "accepted": s.accepted},
            "diagnostics": ds,
        });
        if filter.scoped() {
            out["outside_scope"] =
                serde_json::json!({"errors": outside.errors, "warnings": outside.warnings});
        }
        if filter.skipping() {
            out["skipped"] = serde_json::json!({"warnings": skipped, "codes": filter.skip_codes()});
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print!("{}", diag::render_all(ds, root, color.enabled()));
        print!("{}", found.footer(&filter));
    }
    Ok(i32::from(
        ds.iter().any(|d| d.level == Level::Error && !d.accepted),
    ))
}
