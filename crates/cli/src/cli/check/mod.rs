//! `brickie check`: print the diagnostics, exit non-zero when a finding exists.
//!
//! Exit code: 0 clean, 1 findings, 2 config or cache problem.

mod output;

use super::args::{CheckArgs, ColorArg, Format};
use anyhow::Result;
use brickie_check::check::{self, Filter, Loaded};
use brickie_check::diag::Color;

pub fn run(args: CheckArgs) -> i32 {
    match check(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    }
}

/// Loads the result; with `--measure`, measures first when the cache is missing or stale.
fn load(args: &CheckArgs) -> Result<Loaded> {
    if args.measure && check::needs_measure(&args.config) {
        let path = super::measure::measure(&args.config, None, None)?;
        eprintln!("brickie: wrote {}", path.display());
    }
    check::load(&args.config)
}

fn check(args: &CheckArgs) -> Result<i32> {
    let loaded = load(args)?;
    let filter = Filter::new(&loaded.cfg, &args.bricks)?;
    let found = filter.apply(&loaded);
    let color = match args.color {
        ColorArg::Auto => Color::Auto,
        ColorArg::Always => Color::Always,
        ColorArg::Never => Color::Never,
    };
    match args.format {
        Format::Json => output::json(&filter, &found)?,
        Format::Short => output::short(&loaded, &filter, &found),
        Format::Text => output::text(&loaded, &filter, &found, color),
    }
    Ok(i32::from(!found.shown.is_empty()))
}
