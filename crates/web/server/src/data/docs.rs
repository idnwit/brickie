//! A brick's documentation page data: the brick tree, its children, and its declared items with
//! full docs and a few real use sites per using brick.

use super::diagnostics::Lines;
use super::interface::site;
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_check::interface::{self, IfaceItem};
use brickie_web_api::{DocsExample, DocsItem, DocsNode, DocsPage, DocsUse};
use std::path::{Path, PathBuf};

/// Use sites shown per using brick.
const EXAMPLES: usize = 3;

/// Pre-order nodes of the bricks below `cfg`'s level.
fn walk(cfg: &Config, depth: u32, out: &mut Vec<DocsNode>) {
    for b in &cfg.bricks {
        let mut path = cfg.path.clone();
        path.push(b.name.clone());
        out.push(DocsNode {
            path: level_name(&path),
            name: b.name.clone(),
            depth,
            desc: b.desc.clone(),
            exported: depth == 1 || cfg.exports(&b.name),
        });
        if let Some(sub) = &b.sub {
            walk(sub, depth + 1, out);
        }
    }
}

fn item(i: &IfaceItem, lines: &mut Lines) -> DocsItem {
    let uses = i
        .uses
        .iter()
        .map(|g| DocsUse {
            brick: g.brick.clone(),
            count: g.sites.len(),
            examples: g
                .sites
                .iter()
                .take(EXAMPLES)
                .map(|s| DocsExample {
                    site: site(s),
                    code: lines.line(&s.file, s.line).unwrap_or("").trim().into(),
                })
                .collect(),
        })
        .collect();
    DocsItem {
        leaf: i.leaf.clone(),
        kind: i.kind.clone(),
        name: i.name.clone(),
        signature: i.signature.clone(),
        doc: i.doc.clone(),
        file: i.file.clone(),
        line: i.line,
        uses,
    }
}

/// The inner doc of the brick's root module: the one module of the brick whose parent is
/// outside it, or else its one library crate root. `""` when there is no single root.
fn root_doc(l: &Loaded, path: &[String]) -> String {
    let inside = |id: &str| l.resolved.path(id).is_some_and(|p| p.starts_with(path));
    let roots: Vec<&brickie_facts::Module> = l
        .cache
        .modules
        .iter()
        .filter(|m| inside(&m.id))
        .filter(|m| {
            m.id.rsplit_once("::")
                .is_none_or(|(parent, _)| !inside(parent))
        })
        .collect();
    // A brick's own module (named like the brick) wins over a crate root the brick also holds.
    let own = path.last().and_then(|name| {
        l.cache.modules.iter().find(|m| {
            inside(&m.id) && m.id.contains("::") && m.id.rsplit("::").next() == Some(name.as_str())
        })
    });
    let pick = match (own, roots.as_slice()) {
        (Some(m), _) => Some(m),
        (None, [one]) => Some(*one),
        _ => {
            let libs: Vec<_> = roots
                .iter()
                .filter(|m| !m.id.contains(['/', ':']))
                .collect();
            match libs.as_slice() {
                [one] => Some(**one),
                _ => None,
            }
        }
    };
    pick.map(|m| m.doc.clone()).unwrap_or_default()
}

/// The docs page of the brick at `brick` (`root/core/model`, `core/model`, or `root` / `""`
/// for the tree root). An unknown brick is an error.
pub fn docs(l: &Loaded, brick: &str) -> anyhow::Result<DocsPage> {
    let mut tree = Vec::new();
    walk(&l.cfg, 1, &mut tree);
    let trimmed = brick.trim_matches('/');
    let path = if trimmed.is_empty() || trimmed == "root" {
        Vec::new()
    } else {
        interface::parse_path(&l.cfg, trimmed)?
    };
    let level = level_name(&path);
    let prefix = format!("{level}/");
    let depth = path.len() as u32 + 1;
    let children = tree
        .iter()
        .filter(|n| n.depth == depth && n.path.starts_with(&prefix))
        .cloned()
        .collect();
    let items = if path.is_empty() {
        Vec::new()
    } else {
        let mut lines = Lines::new(Path::new(&l.cache.workspace_root));
        interface::interface(&l.cache, &l.cfg, &l.resolved, &path)
            .items
            .iter()
            .map(|i| item(i, &mut lines))
            .collect()
    };
    Ok(DocsPage {
        brick: level,
        desc: l.cfg.brick_at(&path).and_then(|b| b.desc.clone()),
        doc: if path.is_empty() {
            String::new()
        } else {
            root_doc(l, &path)
        },
        tree,
        children,
        items,
    })
}

fn find_readme_in(dir: &Path) -> Option<PathBuf> {
    for name in ["README.md", "readme.md"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// The `README.md` file sitting alongside `path`'s `brickie.toml` (or in a leaf brick's folder),
/// if one exists on disk.
pub fn readme_path(l: &Loaded, path: &[String]) -> Option<PathBuf> {
    if let Some(cfg) = l.cfg.node(path) {
        let dir = Path::new(&l.cache.workspace_root).join(&cfg.file);
        return find_readme_in(dir.parent()?);
    }
    let b = l.cfg.brick_at(path)?;
    if let Some(p) = &b.path
        && let Some(found) = find_readme_in(&b.base.join(p))
    {
        return Some(found);
    }
    for m in &b.members {
        if let Some(rel) = m.strip_prefix("path:")
            && let Some(found) = find_readme_in(&b.base.join(rel))
        {
            return Some(found);
        }
    }
    None
}

/// Whether `path` has a `README.md` file on disk.
pub(super) fn has_readme(l: &Loaded, path: &[String]) -> bool {
    readme_path(l, path).is_some()
}

/// Read the `README.md` markdown of `brick` (`root` / `""` for the root level).
pub fn readme(l: &Loaded, brick: &str) -> anyhow::Result<(String, String)> {
    let trimmed = brick.trim_matches('/');
    let path = if trimmed.is_empty() || trimmed == "root" {
        Vec::new()
    } else {
        interface::parse_path(&l.cfg, trimmed)?
    };
    let level = level_name(&path);
    let file =
        readme_path(l, &path).ok_or_else(|| anyhow::anyhow!("no README.md for brick `{level}`"))?;
    let md = std::fs::read_to_string(&file)?;
    Ok((level, md))
}
