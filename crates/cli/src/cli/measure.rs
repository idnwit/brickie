//! `brickie measure`.

use anyhow::Result;
use brickie_measure as measure;
use std::path::{Path, PathBuf};

/// Runs measure (hints and warnings go to stderr) and returns the cache path. Shared with
/// `check --measure`.
pub fn measure(
    config: &Path,
    scip: Option<PathBuf>,
    workspace: Option<PathBuf>,
) -> Result<PathBuf> {
    let additional_targets = brickie_check::config::load_measure(config)?.additional_targets;
    let notes = |line: &str| {
        if line.starts_with("hint:") || line.starts_with("warning:") {
            eprintln!("{line}");
        }
    };
    measure::run_with(
        &measure::Options {
            config: config.to_path_buf(),
            scip,
            workspace,
            additional_targets,
        },
        &notes,
    )
}

pub fn run(config: PathBuf, scip: Option<PathBuf>, workspace: Option<PathBuf>) -> Result<i32> {
    let path = measure(&config, scip, workspace)?;
    eprintln!("brickie: wrote {}", path.display());
    Ok(0)
}
