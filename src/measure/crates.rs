//! Cargo targets as cache crates.

use super::metadata::Workspace;
use crate::cache::Crate;

fn is_lib_kind(kind: &str) -> bool {
    kind.split(',').any(|k| {
        matches!(
            k,
            "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro"
        )
    })
}

/// A library crate is its package name, other targets are `pkg/kind/target`.
pub fn crate_ids(ws: &Workspace) -> Vec<String> {
    ws.targets
        .iter()
        .map(|t| {
            let pkg = &ws.packages[t.package].name;
            if is_lib_kind(&t.kind) {
                pkg.clone()
            } else {
                format!(
                    "{pkg}/{}/{}",
                    t.kind.split(',').next().unwrap_or(""),
                    t.name
                )
            }
        })
        .collect()
}

pub fn crates(ws: &Workspace, ids: &[String]) -> Vec<Crate> {
    ws.targets
        .iter()
        .zip(ids)
        .map(|(t, id)| Crate {
            id: id.clone(),
            package: ws.packages[t.package].name.clone(),
            version: ws.packages[t.package].version.clone(),
            target: t.name.clone(),
            kind: t.kind.clone(),
            root: t.src_path.clone(),
        })
        .collect()
}
