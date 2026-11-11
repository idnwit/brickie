//! `brickie view`.

use crate::web;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(config: PathBuf, bind: String, port: u16) -> Result<i32> {
    web::run(&web::Options { config, bind, port })?;
    Ok(0)
}
