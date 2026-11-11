//! `brickie measure`.

use super::Args;
use crate::measure;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: &Args) -> Result<i32> {
    let path = measure::run(&measure::Options {
        config: PathBuf::from(args.flag("config").unwrap_or("brickie.toml")),
        scip: args.flag("scip").map(PathBuf::from),
        workspace: args.flag("workspace").map(PathBuf::from),
    })?;
    eprintln!("brickie: wrote {}", path.display());
    Ok(0)
}
