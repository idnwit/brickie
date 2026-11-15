//! `brickie check`: print the diagnostics, exit non-zero when an unaccepted error exists.
//!
//! Exit code: 0 clean, 1 unaccepted error findings, 2 config, cache or baseline problem.

mod output;

use super::args::{CheckArgs, ColorArg, Format};
use anyhow::Result;
use brickie_check::check::{self, Baseline, Filter, Loaded};
use brickie_check::diag::Color;
use brickie_model::Level;

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
    let mut filter = Filter::new(&loaded.cfg, &args.bricks, &args.skip)?;
    if args.new {
        let base = Baseline::read(&args.config)?;
        filter = filter.with_baseline(&args.config, base)?;
    }
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
    if args.save_baseline {
        let path = Baseline::capture(&loaded, &args.config)?.write(&args.config)?;
        eprintln!("brickie: wrote {}", path.display());
    }
    let ds = &found.shown;
    Ok(i32::from(
        ds.iter().any(|d| d.level == Level::Error && !d.accepted),
    ))
}
