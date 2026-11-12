//! `brickie serve`: the reviewer webview and the measure job.

use anyhow::Result;
use brickie_web as web;
use std::path::PathBuf;

pub fn run(config: PathBuf, bind: String, port: u16) -> Result<i32> {
    web::run(&web::Options { config, bind, port })?;
    Ok(0)
}
