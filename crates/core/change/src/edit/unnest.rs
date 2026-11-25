//! `unnest`: undo a composite. Its children and peer groups are lifted into the parent file
//! (`path` and `path:` members rebased), with the child level's `[[rule]]`, `[[link]]` and
//! `[[complexity]]` entries (their selectors name the same bricks, now siblings); the brick stays
//! as a leaf without `path`. The child level's `exports` no longer mean anything and are dropped.
//! The child file leaves the design: publish lists it for deletion by hand (it never deletes).

use super::Docs;
use super::brick::{brick_mut, insert};
use super::group::name_taken;
use super::moves::rebase;
use super::table::{aot, aot_mut};
use crate::paths::{dir_of, join_rel};
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use toml_edit::{Item, Table};

pub fn unnest(docs: &mut Docs, file: &str, name: &str) -> Result<()> {
    super::via::guard_composite(docs, file, name, "unnesting it")?;
    let t = brick_mut(docs.get_mut(file)?, name)?;
    let path = t
        .get("path")
        .and_then(Item::as_str)
        .with_context(|| format!("brick '{name}' is not a composite"))?
        .trim_end_matches('/')
        .to_string();
    t.remove("path");
    let child = join_rel(file, &format!("{path}/brickie.toml"));
    let doc = docs
        .get(&child)
        .with_context(|| format!("{child} is not a config file of this change"))?
        .clone();
    if doc.contains_key("include") {
        bail!("{child} has `include`: remove it before unnesting '{name}'");
    }
    let (from, to) = (dir_of(&child), dir_of(file));
    let lifted = |path: &[&str]| -> Vec<Table> {
        aot(&doc, path).map_or_else(Vec::new, |a| a.iter().cloned().collect())
    };
    let parent = docs.get_mut(file)?;
    for g in lifted(&["group"]) {
        let gname = g
            .get("name")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string();
        if name_taken(parent, &gname) {
            bail!(
                "{DESIGN_INVALID}: unnest '{name}': group '{gname}' collides with an existing \
                 brick or group"
            );
        }
        aot_mut(parent, &["group"])?.push(g);
    }
    for mut b in lifted(&["brick"]) {
        let bname = b
            .get("name")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_string();
        if name_taken(parent, &bname) {
            bail!(
                "{DESIGN_INVALID}: unnest '{name}': brick '{bname}' collides with an existing \
                 brick or group"
            );
        }
        rebase(&mut b, &from, &to);
        insert(parent, b).with_context(|| format!("unnest '{name}'"))?;
    }
    for path in [&["rule"][..], &["link"], &["complexity"]] {
        for t in lifted(path) {
            aot_mut(parent, path)?.push(t);
        }
    }
    super::brick::prune_empty(parent);
    docs.remove(&child);
    Ok(())
}
