//! Brick interfaces and edge sites from the check result, as web api views.

use brickie_check::check::Loaded;
use brickie_check::interface::{self, EdgeItem, IfaceItem, Site};
use brickie_web_api::{EdgeItemView, IfaceItemView, InterfaceView, SiteView, UseGroupView};

fn site(s: &Site) -> SiteView {
    SiteView {
        file: s.file.clone(),
        line: s.line,
        col: s.col,
        end_col: s.end_col,
        scope: s.scope.clone(),
        kind: s.kind.clone(),
        written: s.written.clone(),
        symbol: s.symbol.clone(),
    }
}

fn item(i: &IfaceItem) -> IfaceItemView {
    IfaceItemView {
        leaf: i.leaf.clone(),
        kind: i.kind.clone(),
        name: i.name.clone(),
        signature: i.signature.clone(),
        doc: i.doc.clone(),
        file: i.file.clone(),
        line: i.line,
        uses: i
            .uses
            .iter()
            .map(|g| UseGroupView {
                brick: g.brick.clone(),
                sites: g.sites.iter().map(site).collect(),
            })
            .collect(),
    }
}

fn edge_item(i: &EdgeItem) -> EdgeItemView {
    EdgeItemView {
        module: i.module.clone(),
        name: i.name.clone(),
        kind: i.kind.clone(),
        signature: i.signature.clone(),
        file: i.file.clone(),
        line: i.line,
        sites: i.sites.iter().map(site).collect(),
    }
}

/// The interface of the brick at `brick` (`root/web/api`).
pub fn interface(l: &Loaded, brick: &str) -> anyhow::Result<InterfaceView> {
    let path = interface::parse_path(&l.cfg, brick)?;
    let i = interface::interface(&l.cache, &l.cfg, &l.resolved, &path);
    Ok(InterfaceView {
        brick: i.brick,
        items: i.items.iter().map(item).collect(),
    })
}

/// Every reference site from brick `from` into brick `to`, by used item.
pub fn edge_uses(l: &Loaded, from: &str, to: &str) -> anyhow::Result<Vec<EdgeItemView>> {
    let a = interface::parse_path(&l.cfg, from)?;
    let b = interface::parse_path(&l.cfg, to)?;
    Ok(interface::edge_uses(&l.cache, &l.resolved, &a, &b)
        .iter()
        .map(edge_item)
        .collect())
}
