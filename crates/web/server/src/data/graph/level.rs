//! One level of the tree: children (rows by first tag), faded outside neighbours and the edges
//! aggregated by (from, to, kind) with their findings.

use super::edges::{End, ModEdge, end, mod_edges};
use super::heat::heat;
use brickie_check::check as rules;
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_check::diag::Diagnostic;
use brickie_web_api::*;
use std::collections::{BTreeMap, BTreeSet};

/// Row captions of the outside neighbours: sources first, targets last.
pub const OUT_SOURCES: &str = "inbound";
pub const OUT_TARGETS: &str = "outbound";
/// What every level needs, computed once per page.
pub struct Ctx<'a> {
    l: &'a Loaded,
    edges: Vec<ModEdge>,
    heat: BTreeMap<String, (f64, Vec<HotFn>)>,
    diags: BTreeMap<&'a str, &'a Diagnostic>,
    links: Vec<(String, UnusedLink)>,
}

#[derive(Default)]
struct Acc {
    prod: bool,
    refs: usize,
    ids: BTreeSet<String>,
}

/// Leaf paths of outside neighbours.
type Paths<'a> = BTreeSet<&'a Vec<String>>;

fn outside_path(x: &End) -> Option<&Vec<String>> {
    match x {
        End::Outside(p) => Some(p),
        End::Child(_) => None,
    }
}

/// The module edges of one level folded by (from, to, kind).
struct Agg {
    edges: BTreeMap<(End, End, String), Acc>,
    /// Per independent target, the ends whose clean edges into it are left out (no ghost, no
    /// edge, no count).
    hidden: BTreeMap<End, BTreeSet<End>>,
}

impl Agg {
    /// Edges with a child of `level` at either end; both ends outside are not this level's.
    fn new(edges: &[ModEdge], level: &[String]) -> Agg {
        let mut agg: BTreeMap<(End, End, String), Acc> = BTreeMap::new();
        let mut hidden: BTreeMap<End, BTreeSet<End>> = BTreeMap::new();
        for e in edges {
            let (Some(a), Some(b)) = (end(&e.from, level), end(&e.to, level)) else {
                continue;
            };
            let inside = |x: &End| matches!(x, End::Child(_));
            if a == b || !(inside(&a) || inside(&b)) {
                continue;
            }
            if e.hidden {
                hidden.entry(b).or_default().insert(a);
                continue;
            }
            let acc = agg.entry((a, b, e.kind.clone())).or_default();
            acc.prod |= e.prod;
            acc.refs += e.refs;
            acc.ids.extend(e.diags.iter().cloned());
        }
        Agg { edges: agg, hidden }
    }

    /// The outside neighbours: those that are an edge source, and all of them.
    fn outside(&self) -> (Paths<'_>, Paths<'_>) {
        let keys = || self.edges.keys();
        (
            keys().filter_map(|(a, _, _)| outside_path(a)).collect(),
            keys()
                .flat_map(|(a, b, _)| [a, b])
                .filter_map(outside_path)
                .collect(),
        )
    }
}

impl<'a> Ctx<'a> {
    /// `links`: the rule links to draw ([`super::links::served`] or [`super::links::rules`]).
    pub fn new(l: &'a Loaded, links: Vec<(String, UnusedLink)>) -> Ctx<'a> {
        Ctx {
            l,
            edges: mod_edges(l),
            heat: heat(l),
            diags: l.diagnostics.iter().map(|d| (d.id.as_str(), d)).collect(),
            links,
        }
    }

    pub fn level(&self, cfg: &Config) -> LevelView {
        let name = cfg.level();
        let agg = Agg::new(&self.edges, &cfg.path);
        let (sources, outside) = agg.outside();
        let mut rows: Vec<String> = Vec::new();
        if !sources.is_empty() {
            rows.push(OUT_SOURCES.into());
        }
        let mut bricks = self.children(cfg, &agg, &mut rows);
        bricks.extend(self.ghosts(&mut rows, &sources, &outside));
        let index = index_of(&bricks, &outside, cfg.bricks.len());
        let hops = cycle_hops(&self.l.diagnostics, &name, &index);
        let edges = (agg.edges.into_iter())
            .map(|(key, acc)| self.edge_view(key, acc, &index, &hops, &name))
            .collect();
        let unused_links = (self.links.iter())
            .filter(|(lv, _)| *lv == name)
            .map(|(_, e)| e.clone())
            .collect();
        LevelView {
            path: name,
            file: cfg.file.clone(),
            exports: cfg.exports.clone(),
            graph: GraphView {
                rows,
                bricks,
                edges,
            },
            unused_links,
        }
    }

    /// The children of `cfg`, each in the row of its first tag.
    fn children(&self, cfg: &Config, agg: &Agg, rows: &mut Vec<String>) -> Vec<BrickView> {
        let node = |b: &brickie_check::config::Brick| {
            let mut path = cfg.path.clone();
            path.push(b.name.clone());
            let tag = (b.tags.first().cloned()).unwrap_or_else(|| "(untagged)".into());
            let row = row_of(rows, &tag);
            BrickView {
                tag,
                row,
                tags: b.tags.clone(),
                members: b.members.clone(),
                hidden_in: (agg.hidden.get(&End::Child(b.name.clone()))).map_or(0, |s| s.len()),
                exported: cfg.exports(&b.name),
                ..self.node(&path, b.name.clone())
            }
        };
        cfg.bricks.iter().map(node).collect()
    }

    /// The faded outside neighbours: sources in the inbound row, the rest in the outbound row.
    fn ghosts(&self, rows: &mut Vec<String>, sources: &Paths, outside: &Paths) -> Vec<BrickView> {
        let mut out = Vec::new();
        for p in outside {
            let label = if sources.contains(p) {
                OUT_SOURCES
            } else {
                OUT_TARGETS
            };
            let row = row_of(rows, label);
            out.push(BrickView {
                tag: label.into(),
                row,
                ghost: true,
                ..self.node(p, level_name(p))
            });
        }
        out
    }

    /// One aggregated edge with the findings of its module edges (plus the cycles through it).
    fn edge_view(
        &self,
        (a, b, kind): (End, End, String),
        mut acc: Acc,
        index: &BTreeMap<End, usize>,
        hops: &BTreeMap<(usize, usize), Vec<String>>,
        level: &str,
    ) -> EdgeView {
        let (from, to) = (index[&a], index[&b]);
        if acc.prod {
            acc.ids
                .extend(hops.get(&(from, to)).into_iter().flatten().cloned());
        }
        let found: Vec<&Diagnostic> = acc
            .ids
            .iter()
            .filter_map(|id| self.diags.get(id.as_str()).copied())
            .collect();
        let wall = matches!(a, End::Outside(_))
            && found
                .iter()
                .any(|d| d.code == rules::DEPS_PRIVATE && d.brick_level.as_deref() == Some(level));
        EdgeView {
            from,
            to,
            kind,
            test: !acc.prod,
            diags: acc.ids.into_iter().collect(),
            count: acc.refs,
            wall,
            under: Vec::new(),
        }
    }

    /// The tree node at `path` as a brick named `name` (row and tags filled by the caller).
    fn node(&self, path: &[String], name: String) -> BrickView {
        let key = level_name(path);
        let (worst_ratio, hot) = self.heat.get(&key).cloned().unwrap_or_default();
        let sub = self.l.cfg.node(path);
        BrickView {
            name,
            path: key,
            desc: self.l.cfg.brick_at(path).and_then(|b| b.desc.clone()),
            independent: matches!(path, [_])
                && self.l.cfg.brick_at(path).is_some_and(|b| b.independent),
            worst_ratio,
            hot,
            children: sub
                .map(|s| s.bricks.iter().map(|b| b.name.clone()).collect())
                .unwrap_or_default(),
            exports: sub.map(|s| s.exports.clone()).unwrap_or_default(),
            file: sub.map(|s| s.file.clone()),
            ..Default::default()
        }
    }
}

/// Brick index per end. A ghost's key is read off `outside` at `i - children` (ghosts follow
/// the `children` children in `bricks`, in `outside` order).
fn index_of(bricks: &[BrickView], outside: &Paths, children: usize) -> BTreeMap<End, usize> {
    let key = |i: usize, b: &BrickView| {
        if b.ghost {
            let p = outside.iter().nth(i - children).cloned().cloned();
            End::Outside(p.unwrap_or_default())
        } else {
            End::Child(b.name.clone())
        }
    };
    (bricks.iter().enumerate())
        .map(|(i, b)| (key(i, b), i))
        .collect()
}

fn row_of(rows: &mut Vec<String>, tag: &str) -> usize {
    rows.iter().position(|r| r == tag).unwrap_or_else(|| {
        rows.push(tag.to_string());
        rows.len() - 1
    })
}

/// Child index pairs on the path of each `deps/cycle` finding of level `level`, with the ids.
pub fn cycle_hops(
    diags: &[Diagnostic],
    level: &str,
    index: &BTreeMap<End, usize>,
) -> BTreeMap<(usize, usize), Vec<String>> {
    let mut out: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    let at =
        |d: &&Diagnostic| d.code == rules::DEPS_CYCLE && d.brick_level.as_deref() == Some(level);
    for d in diags.iter().filter(at) {
        for h in &d.edges {
            let ix = |n: &str| index.get(&End::Child(n.to_string())).copied();
            if let (Some(f), Some(t)) = (ix(&h.from), ix(&h.to)) {
                out.entry((f, t)).or_default().push(d.id.clone());
            }
        }
    }
    out
}
