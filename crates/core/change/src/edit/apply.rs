//! Apply one op to the documents.

use super::Docs;
use super::brick::{self, Fields};
use super::rules::{complexity_table, edit_include, edit_list, rule_table, set_deps, set_exports};
use crate::paths::join_rel;
use anyhow::{Result, bail};
use brickie_model::change::Op;
use toml_edit::value;

const RULES: &[&str] = &["deps", "rule"];
const COMPLEXITY: &[&str] = &["complexity"];

pub fn apply(docs: &mut Docs, op: &Op) -> Result<()> {
    if let Op::CreateFile { file } = op {
        docs.create(file)?;
        return Ok(());
    }
    match op {
        Op::MakeComposite { file, name, path } => return make_composite(docs, file, name, path),
        Op::AddBrick { file, brick } => return super::moves::add_brick(docs, file, brick),
        Op::MoveBrick { .. } => return super::moves::move_brick(docs, op),
        _ => {}
    }
    let doc = docs.get_mut(op.file())?;
    match op {
        Op::RemoveBrick { name, .. } => brick::remove(doc, name),
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
        Op::RenameBrick { from, to, .. } => brick::rename(doc, from, to),
        Op::SetExports { exports, .. } => {
            set_exports(doc, exports);
            Ok(())
        }
        Op::AddRule { rule, .. } => edit_list(doc, RULES, None, Some(rule_table(rule))),
        Op::RemoveRule { index, .. } => edit_list(doc, RULES, Some(*index), None),
        Op::SetRule { index, rule, .. } => {
            edit_list(doc, RULES, Some(*index), Some(rule_table(rule)))
        }
        Op::AddComplexity { rule, .. } => {
            edit_list(doc, COMPLEXITY, None, Some(complexity_table(rule)))
        }
        Op::RemoveComplexity { index, .. } => edit_list(doc, COMPLEXITY, Some(*index), None),
        Op::SetComplexity { index, rule, .. } => {
            edit_list(doc, COMPLEXITY, Some(*index), Some(complexity_table(rule)))
        }
        Op::SetDeps {
            default, cycles, ..
        } => set_deps(doc, default.as_deref(), cycles.as_deref()),
        Op::AddInclude { path, .. } => edit_include(doc, path, true),
        Op::RemoveInclude { path, .. } => edit_include(doc, path, false),
        Op::CreateFile { .. }
        | Op::MakeComposite { .. }
        | Op::AddBrick { .. }
        | Op::MoveBrick { .. } => unreachable!("handled above"),
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
