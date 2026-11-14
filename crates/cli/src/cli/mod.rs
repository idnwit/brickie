//! Command line: argument parsing and one file per command.

mod args;
mod check;
mod design;
mod interface;
mod llms;
mod measure;
mod serve;

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
        } => Ok(check::run(&config, format, color)),
        Command::Design(args) => Ok(design::run(*args)),
        Command::Interface {
            brick,
            config,
            json,
            from,
        } => interface::run(&config, &brick, from.as_deref(), json),
        Command::Serve { config, port, bind } => serve::run(config, bind, port),
        Command::LlmsTxt => llms::run(),
    }
}
