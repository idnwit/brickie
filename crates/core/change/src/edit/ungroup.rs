//! `ungroup`: undo a composite. Its children are lifted into the parent file (`path` and
//! `path:` members rebased), with the child level's `[[deps.rule]]` and `[[complexity]]`
//! entries (their selectors name the same bricks, now siblings); the brick stays as a leaf
//! without `path`. The child level's `exports` no longer mean anything and are dropped. The
//! child file leaves the design: publish lists it for deletion by hand (it never deletes).

use super::Docs;
use super::brick::{brick_mut, insert};
use super::moves::rebase;
use super::table::{aot, aot_mut};
use crate::paths::{dir_of, join_rel};
use anyhow::{Context, Result, bail};
use toml_edit::{Item, Table};

pub fn ungroup(docs: &mut Docs, file: &str, name: &str) -> Result<()> {
    super::via::guard_composite(docs, file, name, "ungrouping it")?;
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
        bail!("{child} has `include`: remove it before ungrouping '{name}'");
    }
    let deps = doc.get("deps").and_then(Item::as_table);
    if deps.is_some_and(|d| d.contains_key("default") || d.contains_key("cycles")) {
        bail!("{child} sets [deps] default/cycles: remove them before ungrouping '{name}'");
    }
    let (from, to) = (dir_of(&child), dir_of(file));
    let lifted = |path: &[&str]| -> Vec<Table> {
        aot(&doc, path).map_or_else(Vec::new, |a| a.iter().cloned().collect())
    };
    let parent = docs.get_mut(file)?;
    for mut b in lifted(&["brick"]) {
        rebase(&mut b, &from, &to);
        insert(parent, b).with_context(|| format!("ungroup '{name}'"))?;
    }
    for path in [&["deps", "rule"][..], &["complexity"]] {
        for t in lifted(path) {
            aot_mut(parent, path)?.push(t);
        }
    }
    super::brick::prune_empty(parent);
    docs.remove(&child);
    Ok(())
}
