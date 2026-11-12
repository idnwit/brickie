//! Command line: argument parsing and one file per command.

mod args;
mod change;
mod check;
mod interface;
mod llms;
mod measure;
mod view;

use anyhow::Result;
use args::{Cli, Command};
use clap::Parser;

/// Parses the process arguments, runs the command and returns the process exit code.
pub fn run() -> Result<i32> {
    match Cli::parse().command {
        Command::Measure {
            config,
            scip,
            workspace,
        } => measure::run(config, scip, workspace),
        Command::Check {
            config,
            format,
            color,
            change,
        } => Ok(check::run(&config, format, color, change)),
        Command::Change(args) => Ok(change::run(args)),
        Command::Interface {
            brick,
            config,
            json,
            from,
        } => interface::run(&config, &brick, from.as_deref(), json),
        Command::View { config, port, bind } => view::run(config, bind, port),
        Command::LlmsTxt => llms::run(),
    }
}
