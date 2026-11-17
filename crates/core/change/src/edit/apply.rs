//! Apply one op to the documents in memory.

use super::Docs;
use super::brick::{self, Fields};
use super::find;
use super::rules::{
    COMPLEXITY, LINKS, RULES, complexity_table, edit_include, edit_list, patch_complexity,
    patch_rule, read_exports, rule_table, set_exports,
};
use super::via;
use crate::paths::join_rel;
use anyhow::{Result, bail};
use brickie_model::design::Op;
use toml_edit::{DocumentMut, value};

pub fn apply(docs: &mut Docs, op: &Op) -> Result<()> {
    apply_one(docs, op)?;
    // Making a brick composite with `set --path` scaffolds its level file, as `add` does.
    if let Op::SetBrick {
        file,
        path: Some(p),
        ..
    } = op
    {
        let p = p.trim_end_matches('/');
        docs.ensure_level(&join_rel(file, &format!("{p}/brickie.toml")))?;
    }
    Ok(())
}

fn apply_one(docs: &mut Docs, op: &Op) -> Result<()> {
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
        Op::AddExport { file, brick } => {
            let doc = docs.get_mut(file)?;
            let mut list = read_exports(doc);
            if !list.contains(brick) {
                list.push(brick.clone());
            }
            set_exports(doc, &list);
            return Ok(());
        }
        Op::RemoveExport { file, brick } => {
            if read_exports(docs.get_mut(file)?).contains(brick) {
                via::guard_child(docs, file, brick, "unexporting it")?;
            }
            let doc = docs.get_mut(file)?;
            let mut list = read_exports(doc);
            list.retain(|e| e != brick);
            set_exports(doc, &list);
            return Ok(());
        }
        _ => {}
    }
    apply_to_doc(docs.get_mut(op.file())?, op)
}

/// The ops that edit the one document they name.
fn apply_to_doc(doc: &mut DocumentMut, op: &Op) -> Result<()> {
    match op {
        Op::SetBrick {
            name,
            desc,
            tags,
            members,
            path,
            independent,
            ..
        } => brick::set(
            doc,
            name,
            Fields {
                desc: desc.as_deref(),
                tags: tags.as_deref(),
                members: members.as_deref(),
                path: path.as_deref(),
                independent: *independent,
            },
        ),
        Op::AddRule { file, rule: r } | Op::AddLink { file, link: r } => {
            let t = table(op);
            find::new_rule(doc, t, file, r)?;
            edit_list(doc, t, None, Some(rule_table(r)))
        }
        Op::RemoveRule { file, key } | Op::RemoveLink { file, key } => {
            let i = find::rule(doc, table(op), file, key)?;
            edit_list(doc, table(op), Some(i), None)
        }
        Op::SetRule { file, key, set } | Op::SetLink { file, key, set } => {
            let i = find::rule(doc, table(op), file, key)?;
            patch_rule(doc, table(op), i, set)
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
        Op::AddInclude { path, .. } => edit_include(doc, path, true),
        Op::RemoveInclude { path, .. } => edit_include(doc, path, false),
        Op::CreateFile { .. }
        | Op::RemoveBrick { .. }
        | Op::RenameBrick { .. }
        | Op::AddExport { .. }
        | Op::RemoveExport { .. }
        | Op::MakeComposite { .. }
        | Op::AddBrick { .. }
        | Op::MoveBrick { .. }
        | Op::Ungroup { .. } => unreachable!("handled above"),
    }
}

/// The array of tables a rule or link op edits.
fn table(op: &Op) -> &'static [&'static str] {
    match op {
        Op::AddLink { .. } | Op::RemoveLink { .. } | Op::SetLink { .. } => LINKS,
        _ => RULES,
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
    docs.create_level(&join_rel(file, &format!("{path}/brickie.toml")))?;
    Ok(())
}
