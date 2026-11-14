//! `brickie measure`.

use anyhow::Result;
use brickie_measure as measure;
use std::path::PathBuf;

pub fn run(config: PathBuf, scip: Option<PathBuf>, workspace: Option<PathBuf>) -> Result<i32> {
    let additional_targets = brickie_check::config::load_measure(&config)?.additional_targets;
    let notes = |line: &str| {
        if line.starts_with("hint:") || line.starts_with("warning:") {
            eprintln!("{line}");
        }
    };
    let path = measure::run_with(
        &measure::Options {
            config,
            scip,
            workspace,
            additional_targets,
        },
        &notes,
    )?;
    eprintln!("brickie: wrote {}", path.display());
    Ok(0)
}
