//! `brickie.review.toml`: findings a human reviewer accepted.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

mod edit;

pub use edit::set_review;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Review {
    #[serde(default)]
    pub accept: Vec<Accept>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accept {
    pub id: String,
    pub reason: String,
}

/// Directory of the config file (where `.brickie/` and the review file live).
pub fn config_dir(config: &Path) -> PathBuf {
    config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

pub fn review_path(config: &Path) -> PathBuf {
    config_dir(config).join("brickie.review.toml")
}

/// Reads the review file next to the config; an absent file is an empty review.
pub fn read(config: &Path) -> Result<Review> {
    let path = review_path(config);
    if !path.exists() {
        return Ok(Review::default());
    }
    let text = std::fs::read_to_string(&path)?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
