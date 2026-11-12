//! One level of the tree: children (rows by first tag), faded outside neighbours and the edges
//! aggregated by (from, to, kind) with their findings.

use super::edges::{End, ModEdge, end, mod_edges};
use super::heat::heat;
use brickie_core::check::Loaded;
use brickie_core::config::{Config, Level, level_name};
use brickie_core::diag::Diagnostic;
use brickie_core::rules;
use brickie_ui::*;
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
}

#[derive(Default)]
struct Acc {
    prod: bool,
    refs: usize,
    ids: BTreeSet<String>,
}

impl<'a> Ctx<'a> {
    pub fn new(l: &'a Loaded) -> Ctx<'a> {
        Ctx {
            l,
            edges: mod_edges(l),
            heat: heat(l),
            diags: l.diagnostics.iter().map(|d| (d.id.as_str(), d)).collect(),
        }
    }

    pub fn level(&self, cfg: &Config) -> LevelView {
        let name = cfg.level();
        let mut agg: BTreeMap<(End, End, String), Acc> = BTreeMap::new();
        for e in &self.edges {
            let (Some(a), Some(b)) = (end(&e.from, &cfg.path), end(&e.to, &cfg.path)) else {
                continue;
            };
            let inside = |x: &End| matches!(x, End::Child(_));
            if a == b || !(inside(&a) || inside(&b)) {
                continue;
            }
            let acc = agg.entry((a, b, e.kind.clone())).or_default();
            acc.prod |= e.prod;
            acc.refs += e.refs;
            acc.ids.extend(e.diags.iter().cloned());
        }
        let sources: BTreeSet<&Vec<String>> = agg
            .keys()
            .filter_map(|(a, _, _)| match a {
                End::Outside(p) => Some(p),
                End::Child(_) => None,
            })
            .collect();
        let outside: BTreeSet<&Vec<String>> = agg
            .keys()
            .flat_map(|(a, b, _)| [a, b])
            .filter_map(|x| match x {
                End::Outside(p) => Some(p),
                End::Child(_) => None,
            })
            .collect();
        let mut rows: Vec<String> = Vec::new();
        if !sources.is_empty() {
            rows.push(OUT_SOURCES.into());
        }
        let mut bricks: Vec<BrickView> = cfg
            .bricks
            .iter()
            .map(|b| {
                let mut path = cfg.path.clone();
                path.push(b.name.clone());
                let tag = b
                    .tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "(untagged)".into());
                let row = row_of(&mut rows, &tag);
                BrickView {
                    tag,
                    row,
                    tags: b.tags.clone(),
                    members: b.members.clone(),
                    exported: cfg.exports.iter().any(|x| x.brick == b.name),
                    ..self.node(&path, b.name.clone())
                }
            })
            .collect();
        for p in &outside {
            let label = if sources.contains(p) {
                OUT_SOURCES
            } else {
                OUT_TARGETS
            };
            let row = row_of(&mut rows, label);
            bricks.push(BrickView {
                tag: label.into(),
                row,
                ghost: true,
                ..self.node(p, level_name(p))
            });
        }
        let index: BTreeMap<End, usize> = bricks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let key = if b.ghost {
                    let p = outside.iter().nth(i - cfg.bricks.len()).cloned().cloned();
                    End::Outside(p.unwrap_or_default())
                } else {
                    End::Child(b.name.clone())
                };
                (key, i)
            })
            .collect();
        let hops = cycle_hops(&self.l.diagnostics, &name, &index);
        let edges = agg
            .into_iter()
            .map(|((a, b, kind), mut acc)| {
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
                    && found.iter().any(|d| {
                        d.code == rules::DEPS_PRIVATE && d.brick_level.as_deref() == Some(&name)
                    });
                let worst = if found.iter().any(|d| d.level == Level::Error) {
                    Some("error".to_string())
                } else {
                    (!found.is_empty()).then(|| "warn".to_string())
                };
                EdgeView {
                    from,
                    to,
                    kind,
                    test: !acc.prod,
                    diags: acc.ids.into_iter().collect(),
                    count: acc.refs,
                    worst,
                    wall,
                }
            })
            .collect();
        LevelView {
            path: name,
            file: cfg.file.clone(),
            exports: cfg.exports.iter().map(|e| e.brick.clone()).collect(),
            graph: GraphView {
                rows,
                bricks,
                edges,
            },
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
            worst_ratio,
            hot,
            children: sub
                .map(|s| s.bricks.iter().map(|b| b.name.clone()).collect())
                .unwrap_or_default(),
            exports: sub
                .map(|s| s.exports.iter().map(|e| e.brick.clone()).collect())
                .unwrap_or_default(),
            file: sub.map(|s| s.file.clone()),
            ..Default::default()
        }
    }
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
