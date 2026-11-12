//! Characterization tests for `smart_route` (run at the end of `layout`): golden snapshots of
//! `kinds`, `po` and sampled routes from the current implementation.

#[path = "../common/mod.rs"]
mod common;
mod golden;

use brickie_layout::{End, InBrick, InEdge, Input, Layout, layout};
use common::fixture;
use golden::{CAPACITY, DIAGONAL, FIXTURE};

fn snap(l: &Layout, sample: &[usize]) -> String {
    let mut s = String::new();
    for (e, k) in l.kinds.iter().enumerate() {
        s += &format!("k{e} {k:?}\n");
    }
    let mut po: Vec<_> = l.po.iter().collect();
    po.sort_by_key(|((e, end), _)| (*e, *end == End::B));
    for ((e, end), p) in po {
        s += &format!("p{e}{end:?} {:?} {:.3}\n", p.side, p.off);
    }
    for &e in sample {
        match &l.routes[e] {
            None => s += &format!("r{e} none\n"),
            Some(r) => {
                s += &format!("r{e}");
                for c in &r.segs {
                    s += " |";
                    for q in c {
                        s += &format!(" {:.3},{:.3}", q.x, q.y);
                    }
                }
                s += "\n";
            }
        }
    }
    s
}

fn two_rows(n_top: usize, n_bot: usize, pairs: &[(usize, usize)]) -> Input {
    let mut bricks = Vec::new();
    for i in 0..n_top {
        bricks.push(InBrick {
            id: format!("t{i}"),
            row: 0,
        });
    }
    for i in 0..n_bot {
        bricks.push(InBrick {
            id: format!("b{i}"),
            row: 1,
        });
    }
    Input {
        rows: vec!["top".into(), "bottom".into()],
        bricks,
        edges: pairs
            .iter()
            .enumerate()
            .map(|(i, &(a, b))| InEdge {
                id: format!("e{i}"),
                a,
                b,
            })
            .collect(),
    }
}

/// Diagonal: the two bricks sit in different rows and columns (padding bricks keep them apart).
fn diagonal() -> Input {
    two_rows(1, 4, &[(0, 1), (0, 2), (0, 3), (0, 4)])
}

/// A narrow row-1 brick receiving parallel pairs from three bricks, plus same-row edges.
fn capacity() -> Input {
    let mut pairs = Vec::new();
    for t in 0..3 {
        pairs.extend([(t, 3); 7]);
    }
    pairs.extend([(3, 4); 5]);
    two_rows(3, 2, &pairs)
}

const FIXTURE_SAMPLE: [usize; 6] = [0, 8, 19, 22, 28, 32];
const DIAGONAL_SAMPLE: [usize; 4] = [0, 1, 2, 3];
const CAPACITY_SAMPLE: [usize; 5] = [0, 10, 21, 22, 25];

fn check(inp: &Input, sample: &[usize], golden: &str) {
    let got = snap(&layout(inp), sample);
    // Deterministic: a second run gives the identical snapshot.
    assert_eq!(got, snap(&layout(inp), sample));
    assert_eq!(got, golden);
}

#[test]
fn fixture_matches_golden() {
    check(&fixture(), &FIXTURE_SAMPLE, FIXTURE);
}

#[test]
fn diagonal_edges_match_golden() {
    check(&diagonal(), &DIAGONAL_SAMPLE, DIAGONAL);
}

#[test]
fn many_parallel_edges_and_arcs_match_golden() {
    check(&capacity(), &CAPACITY_SAMPLE, CAPACITY);
}
