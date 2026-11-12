//! The explorer trees. "Bricks": the brick tree (composites nest their children), each leaf
//! brick with its crates and their files as a folder tree. "Folders": the cached files by directory. Badges are
//! the open errors and warnings, summed up the tree.

use super::marks::counts;
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use codeview_port::{Badge, Node, NodeKind, View};
use std::collections::{BTreeMap, HashMap};

pub fn views(l: &Loaded) -> Vec<View> {
    let c = counts(l);
    vec![
        View {
            name: "Bricks".into(),
            roots: bricks(l, &c),
        },
        View {
            name: "Folders".into(),
            roots: folders(l, &c),
        },
    ]
}

/// A node of `kind` without a file, whose badge sums its children's.
fn group(id: String, label: String, kind: NodeKind, children: Vec<Node>) -> Node {
    let mut b = Badge::default();
    for c in children.iter().filter_map(|c| c.badge.as_ref()) {
        b.errors += c.errors;
        b.warns += c.warns;
    }
    Node {
        id,
        label,
        path: None,
        badge: (b != Badge::default()).then_some(b),
        children,
        kind,
        tags: Vec::new(),
    }
}

fn file(path: &str, label: String, c: &HashMap<&str, Badge>) -> Node {
    Node {
        id: path.to_string(),
        label,
        path: Some(path.to_string()),
        badge: c.get(path).cloned(),
        children: Vec::new(),
        kind: NodeKind::File,
        tags: Vec::new(),
    }
}

/// Files by leaf path, then crate id: `leaf -> crate -> [(file, label)]`.
type Leaves<'a> = BTreeMap<Vec<String>, BTreeMap<&'a str, Vec<(&'a str, String)>>>;

fn leaves(l: &Loaded) -> Leaves<'_> {
    let module_crate: HashMap<&str, &str> = l
        .cache
        .modules
        .iter()
        .map(|m| (m.id.as_str(), m.krate.as_str()))
        .collect();
    let crate_dir: HashMap<&str, &str> = l
        .cache
        .crates
        .iter()
        .map(|c| (c.id.as_str(), c.dir.as_str()))
        .collect();
    let mut out: Leaves = BTreeMap::new();
    for (path, fd) in &l.cache.files {
        let Some(m) = fd.module.as_deref() else {
            continue;
        };
        let (Some(leaf), Some(krate)) = (l.resolved.path(m), module_crate.get(m)) else {
            continue;
        };
        let dir = crate_dir.get(krate).copied().unwrap_or("");
        let label = path
            .strip_prefix(dir)
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or(path);
        out.entry(leaf.to_vec())
            .or_default()
            .entry(krate)
            .or_default()
            .push((path, label.to_string()));
    }
    out
}

fn level(cfg: &Config, at: &[String], leaves: &Leaves, c: &HashMap<&str, Badge>) -> Vec<Node> {
    cfg.bricks
        .iter()
        .map(|b| {
            let path: Vec<String> = at.iter().cloned().chain([b.name.clone()]).collect();
            let (kind, children) = match &b.sub {
                Some(sub) => (NodeKind::Composite, level(sub, &path, leaves, c)),
                None => (NodeKind::Brick, crates(&path, leaves, c)),
            };
            let mut n = group(level_name(&path), b.name.clone(), kind, children);
            n.tags = b.tags.clone();
            n
        })
        .collect()
}

/// A leaf brick's crates, each with its files as a folder tree.
fn crates(path: &[String], leaves: &Leaves, c: &HashMap<&str, Badge>) -> Vec<Node> {
    let name = level_name(path);
    leaves
        .get(path)
        .into_iter()
        .flatten()
        .map(|(krate, files)| {
            let id = format!("{name}#{krate}");
            let mut root = Dir::default();
            for (f, rel) in files {
                root.insert(f, rel);
            }
            let kids = dir_nodes(&format!("{id}/"), &root, c);
            group(id, (*krate).to_string(), NodeKind::Crate, kids)
        })
        .collect()
}

fn bricks(l: &Loaded, c: &HashMap<&str, Badge>) -> Vec<Node> {
    level(&l.cfg, &[], &leaves(l), c)
}

/// A directory while building a folder tree.
#[derive(Default)]
struct Dir<'a> {
    dirs: BTreeMap<&'a str, Dir<'a>>,
    /// Files as (cache path, name).
    files: Vec<(&'a str, &'a str)>,
}

impl<'a> Dir<'a> {
    /// Add the file `path`, placed by `rel` (its directories, then its name).
    fn insert(&mut self, path: &'a str, rel: &'a str) {
        match rel.split_once('/') {
            Some((d, rest)) => self.dirs.entry(d).or_default().insert(path, rest),
            None => self.files.push((path, rel)),
        }
    }
}

/// Folder nodes, then file nodes; folder ids are `prefix` plus their path, ending in `/`.
fn dir_nodes(prefix: &str, d: &Dir, c: &HashMap<&str, Badge>) -> Vec<Node> {
    let mut out: Vec<Node> = d
        .dirs
        .iter()
        .map(|(name, sub)| {
            let p = format!("{prefix}{name}/");
            group(
                p.clone(),
                (*name).to_string(),
                NodeKind::Folder,
                dir_nodes(&p, sub, c),
            )
        })
        .collect();
    out.extend(
        d.files
            .iter()
            .map(|(f, name)| file(f, (*name).to_string(), c)),
    );
    out
}

fn folders(l: &Loaded, c: &HashMap<&str, Badge>) -> Vec<Node> {
    let mut root = Dir::default();
    for path in l.cache.files.keys() {
        root.insert(path, path);
    }
    dir_nodes("", &root, c)
}
