//! `brickie view`.

use super::Args;
use crate::web;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn run(args: &Args) -> Result<i32> {
    web::run(&web::Options {
        config: PathBuf::from(args.flag("config").unwrap_or("brickie.toml")),
        bind: args.flag("bind").unwrap_or("127.0.0.1").to_string(),
        port: args
            .flag("port")
            .unwrap_or("7881")
            .parse()
            .context("--port")?,
    })?;
    Ok(0)
}
