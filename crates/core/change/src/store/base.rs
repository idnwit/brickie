//! The base snapshot: raw bytes of every file of the config tree when the change was opened,
//! stored under `base/` by workspace-relative path (`..` encoded as `^`).

use crate::paths::join_rel;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Every file reachable from the root config (includes and composite configs) plus the
/// review file, read from disk.
pub fn snapshot(root: &Path, config: &str) -> Result<BTreeMap<String, Vec<u8>>> {
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

fn encode(rel: &str) -> String {
    rel.split('/')
        .map(|c| if c == ".." { "^" } else { c })
        .collect::<Vec<_>>()
        .join("/")
}

fn decode(rel: &str) -> String {
    rel.split('/')
        .map(|c| if c == "^" { ".." } else { c })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn write(dir: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    super::lock::write_atomic(&dir.join(encode(rel)), bytes)
}

pub fn read(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out)?;
    Ok(out)
}

fn walk(top: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for e in rd {
        let p = e?.path();
        if p.is_dir() {
            walk(top, &p, out)?;
            continue;
        }
        let rel = crate::paths::slash(p.strip_prefix(top).context("base path")?);
        out.insert(decode(&rel), std::fs::read(&p)?);
    }
    Ok(())
}
