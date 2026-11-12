//! The element an op edits (`key`) and its canonical TOML in a document.

use super::Docs;
use super::table::{aot, canon, find_named};
use brickie_model::change::Op;
use toml_edit::DocumentMut;

/// The element key of an op within its file, given the documents before the op (an added rule
/// gets the next index).
pub fn key(docs: &Docs, op: &Op) -> String {
    let len = |path: &[&str]| {
        docs.get(op.file())
            .and_then(|d| aot(d, path))
            .map_or(0, |a| a.len())
    };
    match op {
        Op::AddBrick { brick, .. } => format!("brick:{}", brick.name),
        Op::RemoveBrick { name, .. }
        | Op::SetBrick { name, .. }
        | Op::MakeComposite { name, .. }
        | Op::Ungroup { name, .. }
        | Op::MoveBrick { name, .. } => format!("brick:{name}"),
        Op::RenameBrick { from, .. } => format!("brick:{from}"),
        Op::SetExports { .. } => "exports".into(),
        Op::AddRule { .. } => format!("rule:{}", len(&["deps", "rule"])),
        Op::RemoveRule { index, .. } | Op::SetRule { index, .. } => format!("rule:{index}"),
        Op::AddComplexity { .. } => format!("complexity:{}", len(&["complexity"])),
        Op::RemoveComplexity { index, .. } | Op::SetComplexity { index, .. } => {
            format!("complexity:{index}")
        }
        Op::SetDeps { .. } => "deps".into(),
        Op::AddInclude { .. } | Op::RemoveInclude { .. } => "include".into(),
        Op::CreateFile { .. } => "file".into(),
    }
}

/// Canonical TOML of element `key` of `file`; `None` when the element (or file) is absent.
pub fn canonical(docs: &Docs, file: &str, key: &str) -> Option<String> {
    let doc = docs.get(file)?;
    if key == "file" {
        return Some(String::new());
    }
    let (kind, arg) = key.split_once(':').unwrap_or((key, ""));
    match kind {
        "brick" => {
            let bricks = aot(doc, &["brick"])?;
            let t = bricks.get(find_named(bricks, arg)?)?;
            Some(canon(t.iter()))
        }
        "rule" => indexed(doc, &["deps", "rule"], arg),
        "complexity" => indexed(doc, &["complexity"], arg),
        "deps" => {
            let deps = doc.get("deps")?.as_table()?;
            let items = deps
                .iter()
                .filter(|(k, i)| *k != "rule" && !i.is_array_of_tables());
            Some(canon(items))
        }
        _ => doc.get(kind).map(|i| canon(std::iter::once((kind, i)))),
    }
}

fn indexed(doc: &DocumentMut, path: &[&str], arg: &str) -> Option<String> {
    let list = aot(doc, path)?;
    list.get(arg.parse().ok()?).map(|t| canon(t.iter()))
}
