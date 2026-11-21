//! `publish`: the draft's way into the published tree.

use super::render;
use anyhow::Result;
use brickie_change::{By, Design};

pub fn publish(d: &Design, json: bool, base: u64, note: Option<String>) -> Result<()> {
    let out = d.publish(base, By::Agent, note)?;
    super::print(json, &render::outcome_json(&out), || render::outcome(&out))
}
