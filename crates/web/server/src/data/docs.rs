//! A brick's documentation page data: the brick tree, its children, and its declared items with
//! full docs and a few real use sites per using brick.

use super::diagnostics::Lines;
use super::interface::site;
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_check::interface::{self, IfaceItem};
use brickie_web_api::{DocsExample, DocsItem, DocsNode, DocsPage, DocsUse};
use std::path::Path;

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
        tree,
        children,
        items,
    })
}
