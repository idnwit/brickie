use super::*;

fn graph(edges: &[(&'static str, &'static str)]) -> Graph<&'static str> {
    let mut g = Graph::new();
    for &(a, b) in edges {
        g.entry(a).or_default().insert(b);
        g.entry(b).or_default();
    }
    g
}

#[test]
fn components_and_cycle_path() {
    let g = graph(&[("a", "b"), ("b", "c"), ("c", "a"), ("c", "d")]);
    let comps = sccs(&g);
    assert!(comps.contains(&vec!["a", "b", "c"]));
    assert!(comps.contains(&vec!["d"]));
    let set: BTreeSet<_> = ["a", "b", "c"].into();
    assert_eq!(find_cycle(&g, &set, &"a"), ["a", "b", "c", "a"]);
}

#[test]
fn acyclic_graph_has_singletons_only() {
    let g = graph(&[("a", "b"), ("b", "c")]);
    assert!(sccs(&g).iter().all(|c| c.len() == 1));
    let set: BTreeSet<_> = ["a"].into();
    assert_eq!(find_cycle(&g, &set, &"a"), ["a"]);
}
