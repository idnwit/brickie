//! One level of the tree: children (rows by first tag), faded outside neighbours and the edges
//! aggregated by (from, to, kind) with their findings.

use super::agg::{Acc, Agg, Paths};
use super::edges::{End, ModEdge, mod_edges};
use super::heat::heat;
use super::lines::line_counts;
use super::project::projected_links;
use brickie_check::check as rules;
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_check::diag::Diagnostic;
use brickie_web_api::*;
use std::collections::BTreeMap;

/// Row captions of the outside neighbours: sources first, targets last.
pub const OUT_SOURCES: &str = "inbound";
pub const OUT_TARGETS: &str = "outbound";
/// What every level needs, computed once per page.
pub struct Ctx<'a> {
    l: &'a Loaded,
    edges: Vec<ModEdge>,
    heat: BTreeMap<String, (f64, Vec<HotFn>)>,
    lines: BTreeMap<String, (u32, u32)>,
    diags: BTreeMap<&'a str, &'a Diagnostic>,
    links: Vec<(String, LevelLink)>,
}

impl<'a> Ctx<'a> {
    /// `links`: the links to draw ([`super::links::served`]).
    pub fn new(l: &'a Loaded, links: Vec<(String, LevelLink)>) -> Ctx<'a> {
        Ctx {
            l,
            edges: mod_edges(l),
            heat: heat(l),
            lines: line_counts(l),
            diags: l.diagnostics.iter().map(|d| (d.id.as_str(), d)).collect(),
            links,
        }
    }

    pub fn level(&self, cfg: &Config) -> LevelView {
        let name = cfg.level();
        let agg = Agg::new(&self.l.cfg, &self.edges, &cfg.path);
        let (mut sources, mut outside) = agg.outside();
        let projected = projected_links(&self.l.cfg, &self.links, cfg);
        for pr in &projected {
            if pr.inbound {
                sources.insert(pr.outside.clone());
            }
            outside.insert(pr.outside.clone());
        }
        let mut rows: Vec<String> = Vec::new();
        if !sources.is_empty() {
            rows.push(OUT_SOURCES.into());
        }
        let child_row_start = rows.len();
        let mut bricks = self.children(cfg, &agg, &mut rows, child_row_start);
        let unit_count = bricks.len();
        bricks.extend(self.ghosts(&mut rows, &sources, &outside));
        let index = index_of(&bricks, &outside, unit_count);
        let hops = cycle_hops(&self.l.diagnostics, &name, &index);
        let edges = (agg.edges.into_iter())
            .map(|(key, acc)| self.edge_view(key, acc, &index, &hops, &name))
            .collect();
        let links = (self.links.iter())
            .filter(|(lv, _)| *lv == name)
            .map(|(_, e)| e.clone())
            .collect();
        let boundary_links = projected
            .into_iter()
            .filter_map(|pr| {
                let ghost = *index.get(&End::Outside(pr.outside))?;
                let child = pr.child.and_then(|c| index.get(&End::Child(c)).copied());
                Some(BoundaryLink {
                    ghost,
                    inbound: pr.inbound,
                    child,
                    link: pr.link,
                })
            })
            .collect();
        let (code_lines, test_lines) = self.lines.get(&name).copied().unwrap_or_default();
        LevelView {
            path: name,
            file: cfg.file.clone(),
            exports: cfg.exports.clone(),
            graph: GraphView {
                rows,
                bricks,
                edges,
            },
            links,
            boundary_links,
            has_readme: crate::data::has_readme(self.l, &cfg.path),
            code_lines,
            test_lines,
        }
    }

    /// The logical units of `cfg` (ungrouped bricks, then peer groups), each in the row of its
    /// first tag (never sharing the ghost rows).
    fn children(
        &self,
        cfg: &Config,
        agg: &Agg,
        rows: &mut Vec<String>,
        start: usize,
    ) -> Vec<BrickView> {
        let mut out: Vec<BrickView> = Vec::new();
        for b in cfg.bricks.iter().filter(|b| b.group.is_none()) {
            let mut path = cfg.path.clone();
            path.push(b.name.clone());
            let tag = (b.tags.first().cloned()).unwrap_or_else(|| "(untagged)".into());
            let row = row_of_from(rows, &tag, start);
            out.push(BrickView {
                tag,
                row,
                tags: b.tags.clone(),
                members: b.members.clone(),
                hidden_in: (agg.hidden.get(&End::Child(b.name.clone()))).map_or(0, |s| s.len()),
                exported: cfg.exports(&b.name),
                ..self.node(&path, b.name.clone())
            });
        }
        for g in &cfg.groups {
            let mut path = cfg.path.clone();
            path.push(g.name.clone());
            let key = level_name(&path);
            let tag = (g.tags.first().cloned()).unwrap_or_else(|| "(untagged)".into());
            let row = row_of_from(rows, &tag, start);
            let (worst_ratio, hot) = self.heat.get(&key).cloned().unwrap_or_default();
            let (code_lines, test_lines) = self.lines.get(&key).copied().unwrap_or_default();
            let group_members: Vec<GroupMemberView> = cfg
                .group_members(&g.name)
                .map(|mb| self.group_member(cfg, mb, &key, &g.name))
                .collect();
            let internal_edges: Vec<InternalEdgeView> = agg
                .internal
                .iter()
                .filter(|((grp, _, _, _), _)| grp == &g.name)
                .map(|((_, fm, tm, kind), acc)| InternalEdgeView {
                    from_member: fm.clone(),
                    to_member: tm.clone(),
                    kind: kind.clone(),
                    test: !acc.prod,
                    count: acc.refs,
                    diags: acc.ids.iter().cloned().collect(),
                })
                .collect();
            out.push(BrickView {
                name: g.name.clone(),
                path: key,
                desc: g.desc.clone(),
                tag,
                row,
                tags: g.tags.clone(),
                worst_ratio,
                hot,
                exported: cfg.exports(&g.name),
                is_group: true,
                group_members,
                internal_edges,
                code_lines,
                test_lines,
                ..Default::default()
            });
        }
        out
    }

    fn group_member(
        &self,
        cfg: &Config,
        mb: &brickie_check::config::Brick,
        group_key: &str,
        group_name: &str,
    ) -> GroupMemberView {
        let mut mpath = cfg.path.clone();
        mpath.push(mb.name.clone());
        let mkey = level_name(&mpath);
        let (worst_ratio, hot) = self.heat.get(&mkey).cloned().unwrap_or_default();
        let excess = excess_of(&hot);
        let errors = self
            .l
            .diagnostics
            .iter()
            .filter(|d| {
                let lv = d.brick_level.as_deref();
                bricks_hit(&d.bricks, lv, &mkey, &mb.name)
                    || (d.code == rules::DEPS_CYCLE
                        && bricks_hit(&d.bricks, lv, group_key, group_name))
            })
            .count();
        let (code_lines, test_lines) = self.lines.get(&mkey).copied().unwrap_or_default();
        GroupMemberView {
            name: mb.name.clone(),
            path: mkey,
            is_composite: mb.sub.is_some(),
            score: errors as f64 + excess,
            worst_ratio,
            errors,
            has_readme: crate::data::has_readme(self.l, &mpath),
            code_lines,
            test_lines,
        }
    }

    /// The faded outside neighbours: sources in row 0 (`inbound`), the rest in a dedicated
    /// `outbound` row at the end (never merged with a child tag of the same name).
    fn ghosts(&self, rows: &mut Vec<String>, sources: &Paths, outside: &Paths) -> Vec<BrickView> {
        let mut out = Vec::new();
        let mut out_row: Option<usize> = None;
        for p in outside {
            let (label, row) = if sources.contains(p) {
                (OUT_SOURCES, 0)
            } else {
                let r = *out_row.get_or_insert_with(|| {
                    rows.push(OUT_TARGETS.into());
                    rows.len() - 1
                });
                (OUT_TARGETS, r)
            };
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
        let (code_lines, test_lines) = self.lines.get(&key).copied().unwrap_or_default();
        let sub = self.l.cfg.node(path);
        let brick = self.l.cfg.brick_at(path);
        let group = path
            .split_last()
            .and_then(|(n, parent)| self.l.cfg.node(parent)?.group(n));
        BrickView {
            name,
            path: key,
            desc: brick
                .and_then(|b| b.desc.clone())
                .or_else(|| group.and_then(|g| g.desc.clone())),
            independent: brick.is_some_and(|b| b.independent),
            is_group: group.is_some(),
            worst_ratio,
            hot,
            children: sub
                .map(|s| s.bricks.iter().map(|b| b.name.clone()).collect())
                .unwrap_or_default(),
            exports: sub.map(|s| s.exports.clone()).unwrap_or_default(),
            file: sub.map(|s| s.file.clone()),
            has_readme: crate::data::has_readme(self.l, path),
            code_lines,
            test_lines,
            ..Default::default()
        }
    }
}

/// Brick index per end. A ghost's key is read off `outside` at `i - children` (ghosts follow
/// the `children` children in `bricks`, in `outside` order).
fn index_of(bricks: &[BrickView], outside: &Paths, children: usize) -> BTreeMap<End, usize> {
    let key = |i: usize, b: &BrickView| {
        if b.ghost {
            let p = outside.iter().nth(i - children).cloned();
            End::Outside(p.unwrap_or_default())
        } else {
            End::Child(b.name.clone())
        }
    };
    (bricks.iter().enumerate())
        .map(|(i, b)| (key(i, b), i))
        .collect()
}

fn row_of_from(rows: &mut Vec<String>, tag: &str, start: usize) -> usize {
    rows[start..]
        .iter()
        .position(|r| r == tag)
        .map(|i| start + i)
        .unwrap_or_else(|| {
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
