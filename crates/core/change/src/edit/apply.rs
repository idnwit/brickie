//! Apply one op to the documents in memory.

use super::Docs;
use super::brick::{self, Fields};
use super::rules::{
    COMPLEXITY, RULES, complexity_table, edit_include, edit_list, patch_complexity, patch_rule,
    rule_table, set_deps, set_exports,
};
use super::{find, via};
use crate::paths::join_rel;
use anyhow::{Result, bail};
use brickie_model::design::Op;
use toml_edit::value;

pub fn apply(docs: &mut Docs, op: &Op) -> Result<()> {
    if let Op::CreateFile { file } = op {
        docs.create(file)?;
        return Ok(());
    }
    match op {
        Op::MakeComposite { file, name, path } => return make_composite(docs, file, name, path),
        Op::AddBrick { file, brick } => return super::moves::add_brick(docs, file, brick),
        Op::MoveBrick { .. } => return super::moves::move_brick(docs, op),
        Op::Ungroup { file, name } => return super::ungroup::ungroup(docs, file, name),
        Op::RemoveBrick { file, name } => {
            via::guard_child(docs, file, name, "removing it")?;
            return brick::remove(docs.get_mut(file)?, name);
        }
        Op::RenameBrick { file, from, to } => {
            brick::rename(docs.get_mut(file)?, from, to)?;
            return via::rename(docs, file, from, to);
        }
        Op::SetExports { file, exports } => {
            let doc = docs.get_mut(file)?;
            let old = brick::exported(doc);
            for n in old
                .iter()
                .filter(|n| !exports.iter().any(|e| &e.brick == *n))
            {
                via::guard_child(docs, file, n, "unexporting it")?;
            }
            set_exports(docs.get_mut(file)?, exports);
            return Ok(());
        }
        _ => {}
    }
    let doc = docs.get_mut(op.file())?;
    match op {
        Op::SetBrick {
            name,
            desc,
            tags,
            members,
            path,
            ..
        } => brick::set(
            doc,
            name,
            Fields {
                desc: desc.as_deref(),
                tags: tags.as_deref(),
                members: members.as_deref(),
                path: path.as_deref(),
            },
        ),
        Op::AddRule { file, rule } => {
            find::new_rule(doc, file, rule)?;
            edit_list(doc, RULES, None, Some(rule_table(rule)))
        }
        Op::RemoveRule { file, key } => {
            let i = find::rule(doc, file, key)?;
            edit_list(doc, RULES, Some(i), None)
        }
        Op::SetRule { file, key, set } => {
            let i = find::rule(doc, file, key)?;
            patch_rule(doc, i, set)
        }
        Op::AddComplexity { file, rule } => {
            find::new_limit(doc, file, rule.bricks.as_deref())?;
            edit_list(doc, COMPLEXITY, None, Some(complexity_table(rule)))
        }
        Op::RemoveComplexity { file, bricks } => {
            let i = find::limit(doc, file, bricks.as_deref())?;
            edit_list(doc, COMPLEXITY, Some(i), None)
        }
        Op::SetComplexity { file, bricks, set } => {
            let i = find::limit(doc, file, bricks.as_deref())?;
            patch_complexity(doc, i, set)
        }
        Op::SetDeps {
            default, cycles, ..
        } => set_deps(doc, default.as_deref(), cycles.as_deref()),
        Op::AddInclude { path, .. } => edit_include(doc, path, true),
        Op::RemoveInclude { path, .. } => edit_include(doc, path, false),
        Op::CreateFile { .. }
        | Op::RemoveBrick { .. }
        | Op::RenameBrick { .. }
        | Op::SetExports { .. }
        | Op::MakeComposite { .. }
        | Op::AddBrick { .. }
        | Op::MoveBrick { .. }
        | Op::Ungroup { .. } => unreachable!("handled above"),
    }
}

/// Set `path` on the brick and create `<path>/brickie.toml` next to `file`.
fn make_composite(docs: &mut Docs, file: &str, name: &str, path: &str) -> Result<()> {
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        bail!("make_composite needs a path");
    }
    let doc = docs.get_mut(file)?;
    let t = brick::brick_mut(doc, name)?;
    if t.contains_key("path") {
        bail!("brick '{name}' is already a composite");
    }
    t["path"] = value(path);
    docs.create(&join_rel(file, &format!("{path}/brickie.toml")))?;
    Ok(())
}
