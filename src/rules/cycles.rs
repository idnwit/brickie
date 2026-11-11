//! `deps/cycle`: cycles between bricks.

use super::codes::DEPS_CYCLE;
use super::{brick_of, span};
use crate::cache::{Cache, Evidence};
use crate::config::{Action, Config, Level};
use crate::diag::{DiagEdge, Finding, Note, RuleRef, Span};
use std::collections::{BTreeMap, BTreeSet};

struct Cycle {
    bricks: Vec<String>,
    path: Vec<String>,
    /// Every brick edge inside the component, with the first reference of each.
    hops: Vec<DiagEdge>,
}

pub(super) fn cycles(cache: &Cache, cfg: &Config, out: &mut Vec<Box<dyn Finding>>) {
    if cfg.deps.cycles != Action::Deny {
        return;
    }
    // Brick graph on prod edges; the first evidence per ordered pair.
    let mut graph: BTreeMap<String, BTreeMap<String, Evidence>> = BTreeMap::new();
    for e in cache.edges().into_iter().filter(|e| e.scope == "prod") {
        let (Some(a), Some(b)) = (brick_of(cache, cfg, &e.from), brick_of(cache, cfg, &e.to))
        else {
            continue;
        };
        if a.name != b.name
            && let Some(ev) = e.evidence.into_iter().next()
        {
            graph
                .entry(a.name.clone())
                .or_default()
                .entry(b.name.clone())
                .or_insert(ev);
        }
    }
    for scc in sccs(&graph) {
        if scc.len() < 2 {
            continue;
        }
        let set: BTreeSet<&str> = scc.iter().map(String::as_str).collect();
        let path = find_cycle(&graph, &set, &scc[0]);
        // Every brick edge inside the component lies on some cycle, so all of them are
        // reported (the path is one example); evidence is the first reference of the pair.
        let hops = scc
            .iter()
            .flat_map(|a| {
                graph[a]
                    .iter()
                    .filter(|(b, _)| set.contains(b.as_str()))
                    .map(|(b, ev)| DiagEdge {
                        from: a.clone(),
                        to: b.clone(),
                        span: Some(span(ev, Some(format!("`{a}` -> `{b}`")))),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        out.push(Box::new(Cycle {
            bricks: scc,
            path,
            hops,
        }));
    }
}

/// A path from `start` back to `start` inside `set` (breadth-first).
fn find_cycle(
    graph: &BTreeMap<String, BTreeMap<String, Evidence>>,
    set: &BTreeSet<&str>,
    start: &str,
) -> Vec<String> {
    let mut prev: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        for next in graph.get(n).into_iter().flat_map(|m| m.keys()) {
            if !set.contains(next.as_str()) {
                continue;
            }
            if next == start {
                let mut path = vec![start.to_string()];
                let mut cur = n;
                let mut rev = vec![cur];
                while cur != start {
                    cur = prev[cur];
                    rev.push(cur);
                }
                rev.reverse();
                path.extend(rev.into_iter().skip(1).map(str::to_string));
                path.push(start.to_string());
                return path;
            }
            if !prev.contains_key(next.as_str()) {
                prev.insert(next, n);
                queue.push_back(next);
            }
        }
    }
    vec![start.to_string()]
}

/// Strongly connected components (Tarjan); each component's names are sorted.
fn sccs(graph: &BTreeMap<String, BTreeMap<String, Evidence>>) -> Vec<Vec<String>> {
    struct St<'a> {
        g: &'a BTreeMap<String, BTreeMap<String, Evidence>>,
        index: BTreeMap<&'a str, usize>,
        low: BTreeMap<&'a str, usize>,
        stack: Vec<&'a str>,
        on: BTreeSet<&'a str>,
        out: Vec<Vec<String>>,
    }
    fn visit<'a>(s: &mut St<'a>, v: &'a str) {
        let i = s.index.len();
        s.index.insert(v, i);
        s.low.insert(v, i);
        s.stack.push(v);
        s.on.insert(v);
        for w in s.g.get(v).into_iter().flat_map(|m| m.keys()) {
            let w = w.as_str();
            if !s.index.contains_key(w) {
                visit(s, w);
                let l = s.low[v].min(s.low[w]);
                s.low.insert(v, l);
            } else if s.on.contains(w) {
                let l = s.low[v].min(s.index[w]);
                s.low.insert(v, l);
            }
        }
        if s.low[v] == s.index[v] {
            let mut comp = Vec::new();
            while let Some(w) = s.stack.pop() {
                s.on.remove(w);
                comp.push(w.to_string());
                if w == v {
                    break;
                }
            }
            comp.sort();
            s.out.push(comp);
        }
    }
    let mut s = St {
        g: graph,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on: BTreeSet::new(),
        out: Vec::new(),
    };
    for v in graph.keys() {
        if !s.index.contains_key(v.as_str()) {
            visit(&mut s, v);
        }
    }
    s.out
}

impl Finding for Cycle {
    fn code(&self) -> String {
        DEPS_CYCLE.into()
    }
    fn level(&self) -> Level {
        Level::Error
    }
    fn subject(&self) -> String {
        self.bricks.join(",")
    }
    fn message(&self) -> String {
        let names: Vec<String> = self.bricks.iter().map(|b| format!("`{b}`")).collect();
        format!("dependency cycle between bricks {}", names.join(", "))
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
            loc: None,
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
