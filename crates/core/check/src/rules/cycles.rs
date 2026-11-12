//! `deps/cycle`: cycles between bricks.

use super::lift::lift;
use super::span;
use crate::config::{Action, Config, Level, Resolved};
use crate::diag::{DiagEdge, Finding, Note, RuleRef, Span};
use brickie_facts::{Cache, Evidence};
use brickie_model::codes::DEPS_CYCLE;
use std::collections::{BTreeMap, BTreeSet};

struct Cycle {
    /// Level name (`root/web`) and config file of the composite whose children cycle.
    level: String,
    file: String,
    bricks: Vec<String>,
    path: Vec<String>,
    /// Every brick edge inside the component, with the first reference of each.
    hops: Vec<DiagEdge>,
}

type Graph = BTreeMap<String, BTreeMap<String, Evidence>>;

/// Per level, cycles among its children over lifted prod edges (levels with
/// `deps.cycles = "deny"` only).
pub(super) fn cycles(cache: &Cache, cfg: &Config, res: &Resolved, out: &mut Vec<Box<dyn Finding>>) {
    // Level path -> child graph; the first evidence per ordered pair.
    let mut graphs: BTreeMap<Vec<String>, Graph> = BTreeMap::new();
    for e in cache.edges().into_iter().filter(|e| e.scope == "prod") {
        let (Some(p1), Some(p2)) = (res.path(&e.from), res.path(&e.to)) else {
            continue;
        };
        let Some(l) = lift(cfg, p1, p2) else { continue };
        if l.level.deps.cycles != Action::Deny {
            continue;
        }
        if let Some(ev) = e.evidence.into_iter().next() {
            graphs
                .entry(l.level.path.clone())
                .or_default()
                .entry(l.from.name.clone())
                .or_default()
                .entry(l.to.name.clone())
                .or_insert(ev);
        }
    }
    for (path, graph) in &graphs {
        let Some(level) = cfg.node(path) else {
            continue;
        };
        level_cycles(level, graph, out);
    }
}

fn level_cycles(level: &Config, graph: &Graph, out: &mut Vec<Box<dyn Finding>>) {
    let adj: brickie_graph::Graph<String> = graph
        .iter()
        .map(|(a, m)| (a.clone(), m.keys().cloned().collect()))
        .collect();
    for scc in brickie_graph::sccs(&adj) {
        if scc.len() < 2 {
            continue;
        }
        let set: BTreeSet<String> = scc.iter().cloned().collect();
        let path = brickie_graph::find_cycle(&adj, &set, &scc[0]);
        // Every brick edge inside the component lies on some cycle, so all of them are
        // reported (the path is one example); evidence is the first reference of the pair.
        let hops = scc
            .iter()
            .flat_map(|a| {
                graph[a]
                    .iter()
                    .filter(|(b, _)| set.contains(*b))
                    .map(|(b, ev)| DiagEdge {
                        from: a.clone(),
                        to: b.clone(),
                        span: Some(span(ev, Some(format!("`{a}` -> `{b}`")))),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        out.push(Box::new(Cycle {
            level: level.level(),
            file: level.file.clone(),
            bricks: scc,
            path,
            hops,
        }));
    }
}

impl Finding for Cycle {
    fn code(&self) -> String {
        DEPS_CYCLE.into()
    }
    fn level(&self) -> Level {
        Level::Error
    }
    fn subject(&self) -> String {
        if self.level == "root" {
            self.bricks.join(",")
        } else {
            format!("{}:{}", self.level, self.bricks.join(","))
        }
    }
    fn message(&self) -> String {
        let names: Vec<String> = self.bricks.iter().map(|b| format!("`{b}`")).collect();
        format!(
            "dependency cycle between bricks {}{}",
            names.join(", "),
            super::deps::at_level(&self.level)
        )
    }
    fn brick_level(&self) -> Option<String> {
        Some(self.level.clone())
    }
    fn spans(&self) -> Vec<Span> {
        // The first edge is the primary location; every edge is listed in the notes and edges.
        self.hops
            .iter()
            .filter_map(|h| h.span.clone())
            .take(1)
            .collect()
    }
    fn edges(&self) -> Vec<DiagEdge> {
        self.hops.clone()
    }
    fn rule(&self) -> Option<RuleRef> {
        Some(RuleRef {
            name: "deps.cycles = \"deny\"".into(),
            loc: Some(self.file.clone()),
        })
    }
    fn help(&self) -> Option<String> {
        Some("cut the cycle: move the shared type into a trait owned by the lower brick, or invert one dependency".into())
    }
    fn bricks(&self) -> Vec<String> {
        self.bricks.clone()
    }
    fn notes(&self) -> Vec<Note> {
        let mut notes = vec![Note {
            text: format!("cycle: {}", self.path.join(" -> ")),
            span: None,
        }];
        notes.extend(self.hops.iter().map(|h| Note {
            text: format!("`{}` -> `{}` is on a cycle, used at", h.from, h.to),
            span: h.span.clone(),
        }));
        notes
    }
}
