//! The config tree on disk: every file reachable from the root config (includes and composite
//! configs) plus the review file, and its hash.

use crate::paths::join_rel;
use anyhow::Result;
use brickie_model::design::{PlanAction, PlanFile};
use std::collections::BTreeMap;
use std::path::Path;

pub type Files = BTreeMap<String, Vec<u8>>;

/// Every file of the tree, read from disk.
pub fn read(root: &Path, config: &str) -> Result<Files> {
    let mut out = BTreeMap::new();
    let mut queue = vec![config.to_string()];
    while let Some(rel) = queue.pop() {
        if out.contains_key(&rel) {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(&rel)) else {
            continue; // a missing file is a config error the loader reports
        };
        queue.extend(references(&rel, &bytes));
        out.insert(rel, bytes);
    }
    let review = join_rel(config, "brickie.review.toml");
    if let Ok(bytes) = std::fs::read(root.join(&review)) {
        out.insert(review, bytes);
    }
    Ok(out)
}

/// The config files the tree names but the disk lacks (a deleted nested `brickie.toml`).
pub fn missing(root: &Path, files: &Files) -> Vec<String> {
    let mut out: Vec<String> = (files.iter())
        .flat_map(|(rel, bytes)| references(rel, bytes))
        .filter(|r| !files.contains_key(r) && !root.join(r).is_file())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The config files a file names: its includes and its composites' `brickie.toml`.
fn references(rel: &str, bytes: &[u8]) -> Vec<String> {
    let Ok(v) = String::from_utf8_lossy(bytes).parse::<toml::Table>() else {
        return Vec::new();
    };
    let strs = |v: Option<&toml::Value>| -> Vec<String> {
        v.and_then(toml::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out: Vec<String> = strs(v.get("include"))
        .iter()
        .map(|i| join_rel(rel, i))
        .collect();
    let bricks = v.get("brick").and_then(toml::Value::as_array);
    for b in bricks.into_iter().flatten() {
        if let Some(p) = b.get("path").and_then(toml::Value::as_str) {
            let p = p.trim_end_matches('/');
            out.push(join_rel(rel, &format!("{p}/brickie.toml")));
        }
    }
    out
}

/// FNV-1a 64 over sorted `(path, bytes)`, hex. Detects edits; not a security hash.
pub fn hash(files: &Files) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for (p, b) in files {
        eat(p.as_bytes());
        eat(&[0]);
        eat(&(b.len() as u64).to_le_bytes());
        eat(b);
    }
    format!("{h:016x}")
}

/// One `created <dir>/` entry per `brickie.toml` in `new` that `old` lacks (a group's folder).
pub fn created(old: &Files, new: &Files) -> Vec<PlanFile> {
    (new.keys())
        .filter(|p| !old.contains_key(*p))
        .filter_map(|p| p.strip_suffix("brickie.toml"))
        .filter(|dir| !dir.is_empty())
        .map(|dir| PlanFile {
            path: dir.to_string(),
            action: PlanAction::Create,
            note: "(group)".into(),
            content: None,
        })
        .collect()
}
