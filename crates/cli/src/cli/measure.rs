//! `brickie measure`.

use anyhow::Result;
use brickie_measure as measure;
use std::path::PathBuf;

pub fn run(config: PathBuf, scip: Option<PathBuf>, workspace: Option<PathBuf>) -> Result<i32> {
    let path = measure::run(&measure::Options {
        config,
        scip,
        workspace,
    })?;
    eprintln!("brickie: wrote {}", path.display());
    Ok(0)
}
