//! `brickie check`: print the diagnostics, exit non-zero when an unaccepted error exists.
//!
//! With `--change`, the config tree is the open change's design (accepted and proposed ops)
//! and bricks with no measured module are listed as `change/planned`.
//!
//! Exit code: 0 clean, 1 unaccepted error findings, 2 config or cache problem.

use super::args::{ColorArg, Format};
use anyhow::Result;
use brickie_change::{Change, planned_text};
use brickie_check::check::{self, Loaded};
use brickie_check::diag::{self, Color};
use brickie_model::Level;
use brickie_model::codes::CHANGE_PLANNED;
use std::path::Path;

pub fn run(config: &Path, format: Format, color: ColorArg, change: bool) -> i32 {
    let json = matches!(format, Format::Json);
    let color = match color {
        ColorArg::Auto => Color::Auto,
        ColorArg::Always => Color::Always,
        ColorArg::Never => Color::Never,
    };
    match check(config, json, color, change) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    }
}

fn check(config: &Path, json: bool, color: Color, change: bool) -> Result<i32> {
    let (loaded, planned) = if change {
        let preview = Change::load(config)?.preview(true)?;
        let l = preview.check()?;
        let planned = brickie_change::planned(&l.cfg, &l.resolved)
            .iter()
            .map(|p| planned_text(&l.cfg, p))
            .collect();
        (l, planned)
    } else {
        (check::load(config)?, Vec::new())
    };
    let Loaded {
        cache,
        diagnostics: ds,
        ..
    } = loaded;
    let root = Path::new(&cache.workspace_root);

    let s = diag::summarize(&ds);
    if json {
        let out = serde_json::json!({
            "summary": {"errors": s.errors, "warnings": s.warnings, "accepted": s.accepted},
            "diagnostics": ds,
            "planned": planned,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        for p in &planned {
            println!("note[{CHANGE_PLANNED}]: {p}");
        }
        print!("{}", diag::render_all(&ds, root, color.enabled()));
    }
    Ok(i32::from(
        ds.iter().any(|d| d.level == Level::Error && !d.accepted),
    ))
}
