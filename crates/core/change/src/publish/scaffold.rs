//! Scaffolding on publish: for each brick the change adds (or makes composite) whose code does
//! not exist, the folder, a crate (`Cargo.toml`, `src/lib.rs`, a workspace member) or a module
//! file (`<module>/mod.rs` and `pub mod` in the parent), as its members say. Never overwrites.

use super::crates::Workspace;
use crate::paths::{join_dir, to_rel};
use brickie_check::config::{Brick, Config};
use brickie_model::change::{PlanAction, PlanFile};
use std::collections::BTreeSet;
use std::path::Path;

fn file(path: String, action: PlanAction, note: String, content: Option<String>) -> PlanFile {
    PlanFile {
        path,
        action,
        note,
        content,
    }
}

struct Out {
    ws: Workspace,
    files: Vec<PlanFile>,
    members: Vec<String>,
}

pub fn scaffold(root: &Path, cfg: &Config, added: &BTreeSet<String>) -> Vec<PlanFile> {
    let mut out = Out {
        ws: Workspace::read(root),
        files: Vec::new(),
        members: Vec::new(),
    };
    walk(cfg, added, &mut out);
    workspace_members(&mut out);
    let edits = std::mem::take(&mut out.ws.edits);
    for (path, content) in edits {
        let unchanged = std::fs::read_to_string(root.join(&path)).ok() == Some(content.clone());
        if !unchanged {
            out.files.push(file(
                path,
                PlanAction::Edit,
                "scaffold".into(),
                Some(content),
            ));
        }
    }
    out.files
}

fn walk(level: &Config, added: &BTreeSet<String>, out: &mut Out) {
    for b in &level.bricks {
        let dir = to_rel(&out.ws.root, &b.base).unwrap_or_default();
        if added.contains(&b.name) {
            brick(b, &dir, out);
        }
        if let Some(sub) = &b.sub {
            walk(sub, added, out);
        }
    }
}

fn brick(b: &Brick, dir: &str, out: &mut Out) {
    if let Some(p) = &b.path {
        let target = join_dir(dir, p.trim_end_matches('/'));
        if !out.ws.exists(&target) {
            let note = format!("folder of composite {}", b.name);
            out.files.push(file(target, PlanAction::Mkdir, note, None));
        }
        return;
    }
    for m in &b.members {
        if let Some(p) = m.strip_prefix("path:") {
            let target = join_dir(dir, p.trim_end_matches('/'));
            let name = target.rsplit('/').next().unwrap_or(&b.name).to_string();
            new_crate(b, &target, &name, out);
        } else if m.contains("::") {
            module(b, m, out);
        } else if !out.ws.packages.contains_key(m) {
            new_crate(b, &join_dir(dir, &b.name), m, out);
        }
    }
}

fn new_crate(b: &Brick, dir: &str, name: &str, out: &mut Out) {
    let manifest = format!("{dir}/Cargo.toml");
    if out.ws.exists(&manifest) {
        return;
    }
    let toml = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
    );
    let note = format!("crate of brick {}", b.name);
    out.files
        .push(file(manifest, PlanAction::Create, note.clone(), Some(toml)));
    let doc = format!("//! {}\n", b.desc.as_deref().unwrap_or(&b.name));
    let lib = format!("{dir}/src/lib.rs");
    out.files
        .push(file(lib, PlanAction::Create, note, Some(doc)));
    out.members.push(dir.to_string());
}

/// `crate::a::b`: create `<src>/a/b/mod.rs` and add `pub mod b;` to the parent module file.
fn module(b: &Brick, member: &str, out: &mut Out) {
    let mut segs: Vec<&str> = member.split("::").collect();
    let krate = segs.remove(0);
    let skip = |out: &mut Out, why: String| {
        out.files
            .push(file(member.into(), PlanAction::Skip, why, None));
    };
    let Some((_, root_file)) = out.ws.packages.get(krate).cloned() else {
        return skip(
            out,
            format!("crate {krate} is not measured: create it, then measure"),
        );
    };
    let src = root_file
        .rsplit_once('/')
        .map_or("", |(d, _)| d)
        .to_string();
    let rel = segs.join("/");
    let (Some(last), Some(parent)) = (
        segs.last().copied(),
        parent_file(out, &src, &root_file, &segs),
    ) else {
        return skip(out, format!("no parent module file for {member}"));
    };
    if out.ws.exists(&format!("{src}/{rel}.rs")) || out.ws.exists(&format!("{src}/{rel}/mod.rs")) {
        return;
    }
    let doc = format!("//! {}\n", b.desc.as_deref().unwrap_or(&b.name));
    let note = format!("module of brick {}", b.name);
    out.files.push(file(
        format!("{src}/{rel}/mod.rs"),
        PlanAction::Create,
        note,
        Some(doc),
    ));
    if let Some(text) = out.ws.content(&parent) {
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!("pub mod {last};\n"));
    }
}

/// The file declaring the module's parent: the crate root, or `<parent>.rs` / `<parent>/mod.rs`.
fn parent_file(out: &Out, src: &str, root_file: &str, segs: &[&str]) -> Option<String> {
    let (_, parents) = segs.split_last()?;
    if parents.is_empty() {
        return Some(root_file.to_string());
    }
    let p = parents.join("/");
    [format!("{src}/{p}.rs"), format!("{src}/{p}/mod.rs")]
        .into_iter()
        .find(|f| out.ws.exists(f))
}

/// Add new crates to `[workspace] members` of the root `Cargo.toml`.
fn workspace_members(out: &mut Out) {
    let members = std::mem::take(&mut out.members);
    if members.is_empty() {
        return;
    }
    let Some(text) = out.ws.content("Cargo.toml") else {
        let why = "no Cargo.toml next to the config: add the new crates to the workspace".into();
        out.files
            .push(file("Cargo.toml".into(), PlanAction::Skip, why, None));
        return;
    };
    let Ok(mut doc) = text.parse::<toml_edit::DocumentMut>() else {
        return;
    };
    let Some(list) = doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("members"))
        .and_then(toml_edit::Item::as_array_mut)
    else {
        return;
    };
    for m in members {
        if !list.iter().any(|v| v.as_str() == Some(m.as_str())) {
            list.push(m);
        }
    }
    *text = doc.to_string();
}
