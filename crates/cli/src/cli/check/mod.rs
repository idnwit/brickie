//! `brickie check`: print the diagnostics, exit non-zero when a finding exists.
//!
//! It reads the working design: the draft when one exists, else the published files; the first
//! line says which.
//!
//! Exit code: 0 clean, 1 findings, 2 config or cache problem. The todo (planned work) is printed
//! after the findings and never counted.

mod output;

use super::args::{CheckArgs, ColorArg, Format};
use anyhow::Result;
use brickie_change::{Working, working};
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
fn load(args: &CheckArgs) -> Result<(Loaded, Working)> {
    if args.measure && check::needs_measure(&args.config) {
        let path = super::measure::measure(&args.config, None, None)?;
        eprintln!("brickie: wrote {}", path.display());
    }
    working::load(&args.config)
}

fn check(args: &CheckArgs) -> Result<i32> {
    let (loaded, working) = load(args)?;
    let filter = Filter::new(&loaded.cfg, &args.bricks)?;
    let found = filter.apply(&loaded);
    let color = match args.color {
        ColorArg::Auto => Color::Auto,
        ColorArg::Always => Color::Always,
        ColorArg::Never => Color::Never,
    };
    match args.format {
        Format::Json => output::json(&loaded, &working, &filter, &found)?,
        Format::Short => output::short(&loaded, &working, &filter, &found),
        Format::Text => output::text(&loaded, &working, &filter, &found, color),
    }
    Ok(i32::from(!found.shown.is_empty()))
}
