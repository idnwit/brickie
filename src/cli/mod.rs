//! Command line: argument parsing and one file per command.

mod args;
mod check;
mod llms;
mod measure;
mod view;

use anyhow::Result;
use args::{Args, USAGE, parse};

/// Runs the command and returns the process exit code.
pub fn run(argv: impl Iterator<Item = String>) -> Result<i32> {
    let args = parse(argv)?;
    match args.positional.first().map(String::as_str) {
        Some("measure") => measure::run(&args),
        Some("check") => check::run(&args),
        Some("view") => view::run(&args),
        Some("llms.txt") => llms::run(),
        _ => {
            eprintln!("{USAGE}");
            Ok(2)
        }
    }
}
