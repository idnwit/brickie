//! brickie-graph: strongly connected components and cycle paths over small directed graphs.
//!
//! A graph is an adjacency map `node -> successors`; nodes are any ordered, clonable keys.
//! Results are deterministic (ordered maps, sorted components).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// An adjacency map: every node with its successors.
pub type Graph<N> = BTreeMap<N, BTreeSet<N>>;

/// Strongly connected components (Tarjan); each component's nodes are sorted.
pub fn sccs<N: Ord + Clone>(graph: &Graph<N>) -> Vec<Vec<N>> {
    let mut s = Tarjan {
        g: graph,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on: BTreeSet::new(),
        out: Vec::new(),
    };
    for v in graph.keys() {
        if !s.index.contains_key(v) {
            s.visit(v);
        }
    }
    s.out
}

struct Tarjan<'a, N> {
    g: &'a Graph<N>,
    index: BTreeMap<&'a N, usize>,
    low: BTreeMap<&'a N, usize>,
    stack: Vec<&'a N>,
    on: BTreeSet<&'a N>,
    out: Vec<Vec<N>>,
}

impl<'a, N: Ord + Clone> Tarjan<'a, N> {
    fn visit(&mut self, v: &'a N) {
        let i = self.index.len();
        self.index.insert(v, i);
        self.low.insert(v, i);
        self.stack.push(v);
        self.on.insert(v);
        for w in self.g.get(v).into_iter().flatten() {
            let bound = if !self.index.contains_key(w) {
                self.visit(w);
                self.low[w]
            } else if self.on.contains(w) {
                self.index[w]
            } else {
                continue;
            };
            let l = self.low[v].min(bound);
            self.low.insert(v, l);
        }
        if self.low[v] == self.index[v] {
            self.pop_component(v);
        }
    }

    fn pop_component(&mut self, v: &N) {
        let mut comp = Vec::new();
        while let Some(w) = self.stack.pop() {
            self.on.remove(w);
            comp.push(w.clone());
            if w == v {
                break;
            }
        }
        comp.sort();
        self.out.push(comp);
    }
}

/// A shortest path from `start` back to `start` through nodes of `set` (breadth-first):
/// `[start, .., start]`, or just `[start]` when there is none.
pub fn find_cycle<N: Ord + Clone>(graph: &Graph<N>, set: &BTreeSet<N>, start: &N) -> Vec<N> {
    let mut prev: BTreeMap<&N, &N> = BTreeMap::new();
    let mut queue = VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        for next in graph.get(n).into_iter().flatten() {
            if !set.contains(next) {
                continue;
            }
            if next == start {
                return unwind(&prev, start, n);
            }
            if !prev.contains_key(next) {
                prev.insert(next, n);
                queue.push_back(next);
            }
        }
    }
    vec![start.clone()]
}

/// `start -> .. -> last -> start` from the breadth-first predecessor map.
fn unwind<N: Ord + Clone>(prev: &BTreeMap<&N, &N>, start: &N, last: &N) -> Vec<N> {
    let mut rev = vec![last];
    let mut cur = last;
    while cur != start {
        cur = prev[cur];
        rev.push(cur);
    }
    rev.reverse();
    let mut path = vec![start.clone()];
    path.extend(rev.into_iter().skip(1).cloned());
    path.push(start.clone());
    path
}

#[cfg(test)]
mod tests;
