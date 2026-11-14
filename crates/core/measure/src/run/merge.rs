//! Union of the caches of several scip runs (the host and each additional target).

use brickie_facts::{Cache, FileData, Function, Item};
use std::collections::BTreeSet;

/// Add `other` into `base`: files, modules and crates seen only by `other` are added; a file
/// compiled by both gets the union of its edges and items (deduplicated).
pub fn merge(base: &mut Cache, other: Cache) {
    let known: BTreeSet<String> = base.crates.iter().map(|c| c.id.clone()).collect();
    base.crates
        .extend(other.crates.into_iter().filter(|c| !known.contains(&c.id)));
    let known: BTreeSet<String> = base.modules.iter().map(|m| m.id.clone()).collect();
    base.modules
        .extend(other.modules.into_iter().filter(|m| !known.contains(&m.id)));
    for (path, theirs) in other.files {
        match base.files.get_mut(&path) {
            None => {
                base.files.insert(path, theirs);
            }
            Some(mine) if !mine.compiled && theirs.compiled => *mine = theirs,
            Some(mine) if theirs.compiled => union_file(mine, theirs),
            Some(_) => {}
        }
    }
}

fn union_file(mine: &mut FileData, theirs: FileData) {
    let edges: BTreeSet<_> = mine.edges.drain(..).chain(theirs.edges).collect();
    mine.edges = edges.into_iter().collect();
    // Items by identity; a run may derive different state for one item, so keep the wider.
    for item in theirs.items {
        let key = |i: &Item| (i.module.clone(), i.kind.clone(), i.name.clone(), i.line);
        match mine.items.iter_mut().find(|i| key(i) == key(&item)) {
            Some(have) => have.exported |= item.exported,
            None => mine.items.push(item),
        }
    }
    let fkey = |f: &Function| (f.qname.clone(), f.name.clone(), f.line);
    for f in theirs.functions {
        if !mine.functions.iter().any(|g| fkey(g) == fkey(&f)) {
            mine.functions.push(f);
        }
    }
    if mine.module.is_none() {
        mine.module = theirs.module;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_facts::{FileEdge, Measured};

    fn edge(to: &str, line: u32) -> FileEdge {
        FileEdge {
            from: "a".into(),
            to: to.into(),
            kind: "uses".into(),
            scope: "prod".into(),
            line,
            col: 1,
            end_col: 2,
            symbol: "s".into(),
            written: String::new(),
        }
    }

    fn file(compiled: bool, edges: Vec<FileEdge>) -> FileData {
        FileData {
            sha256: String::new(),
            compiled,
            size: 0,
            lines: 0,
            scope: "prod".into(),
            module: None,
            functions: Vec::new(),
            edges,
            items: Vec::new(),
        }
    }

    fn cache(files: Vec<(&str, FileData)>) -> Cache {
        Cache {
            schema_version: 0,
            tool_version: String::new(),
            rust_analyzer: String::new(),
            measured: Measured::default(),
            workspace_root: String::new(),
            git: None,
            crates: Vec::new(),
            modules: Vec::new(),
            files: files.into_iter().map(|(p, f)| (p.to_string(), f)).collect(),
        }
    }

    fn item(name: &str, exported: bool) -> Item {
        Item {
            symbol: format!("m::{name}"),
            module: "m".into(),
            kind: "fn".into(),
            name: name.into(),
            signature: String::new(),
            doc: String::new(),
            line: 3,
            vis: "pub".into(),
            visible_in: "m".into(),
            exported,
        }
    }

    fn func(name: &str) -> Function {
        Function {
            name: name.into(),
            qname: format!("m::{name}"),
            module: None,
            line: 5,
            end_line: 6,
            lines: 2,
            cognitive: 0,
            cyclomatic: 1,
            nesting: 0,
            paths: 1,
            params: 0,
            scope: "prod".into(),
        }
    }

    #[test]
    fn dedupes_items_by_identity_and_unions_functions() {
        let mut a = file(true, vec![]);
        a.items = vec![item("f", false)];
        a.functions = vec![func("f")];
        let mut b = file(true, vec![]);
        b.items = vec![item("f", true)];
        b.functions = vec![func("f"), func("only_b")];
        let mut base = cache(vec![("x.rs", a)]);
        merge(&mut base, cache(vec![("x.rs", b)]));
        let f = &base.files["x.rs"];
        assert_eq!(f.items.len(), 1);
        assert!(f.items[0].exported);
        let names: Vec<_> = f.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["f", "only_b"]);
    }

    #[test]
    fn unions_and_dedupes_edges() {
        let mut base = cache(vec![
            ("x.rs", file(true, vec![edge("b", 1), edge("c", 2)])),
            ("gone.rs", file(false, vec![])),
        ]);
        let other = cache(vec![
            ("x.rs", file(true, vec![edge("c", 2), edge("d", 3)])),
            ("gone.rs", file(true, vec![edge("e", 1)])),
            ("new.rs", file(true, vec![edge("f", 1)])),
        ]);
        merge(&mut base, other);
        let to =
            |p: &str| -> Vec<String> { base.files[p].edges.iter().map(|e| e.to.clone()).collect() };
        assert_eq!(to("x.rs"), ["b", "c", "d"]);
        assert_eq!(to("gone.rs"), ["e"]);
        assert!(base.files["gone.rs"].compiled);
        assert_eq!(to("new.rs"), ["f"]);
    }
}
