//! `brickie check`: print the diagnostics, exit non-zero when an unaccepted error exists.
//!
//! Exit code: 0 clean, 1 unaccepted error findings, 2 config or cache problem.

use super::Args;
use crate::check::{self, Loaded};
use crate::config::Level;
use crate::diag::{self, Color};
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

pub fn run(args: &Args) -> Result<i32> {
    let json = match args.flag("format").unwrap_or("text") {
        "text" => false,
        "json" => true,
        f => bail!("--format must be text or json, got {f}"),
    };
    let color = Color::parse(args.flag("color").unwrap_or("auto"))?;
    let config = PathBuf::from(args.flag("config").unwrap_or("brickie.toml"));
    Ok(match check(&config, json, color) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    })
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
