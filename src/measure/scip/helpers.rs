//! Small pure helpers over SCIP symbols and ranges.

use super::Span;
use scip::types::Occurrence;

pub fn module_path(scip: &str) -> Option<String> {
    if !scip.ends_with('/') {
        return None;
    }
    let sym = scip::symbol::parse_symbol(scip).ok()?;
    let names: Vec<&str> = sym
        .descriptors
        .iter()
        .map(|d| d.name.as_str())
        .filter(|n| *n != "crate")
        .collect();
    Some(names.join("/"))
}

/// Innermost definition (by enclosing range) containing `pos`, excluding
/// `exclude`. `defs` are (body span, symbol index) sorted by start asc, end desc.
pub fn innermost(defs: &[(Span, usize)], pos: (u32, u32), exclude: Option<usize>) -> Option<usize> {
    let mut found = None;
    for &(s, id) in defs {
        if (s.0, s.1) > pos {
            break;
        }
        if pos < (s.2, s.3) && Some(id) != exclude {
            found = Some(id);
        }
    }
    found
}

/// Parent of each definition: innermost def sorted *before* it whose range
/// contains its name position (acyclic even for identical enclosing ranges).
pub fn parents(
    defs: &[(Span, usize)],
    name_pos: impl Fn(usize) -> (u32, u32),
) -> Vec<(usize, Option<usize>)> {
    defs.iter()
        .enumerate()
        .map(|(k, &(_, i))| (i, innermost(&defs[..k], name_pos(i), None)))
        .collect()
}

/// Expected module path of `file` if it belongs to the target rooted at `root`.
pub fn expected_module_path(root: &str, file: &str) -> Option<String> {
    let (dir, name) = root.rsplit_once('/').unwrap_or(("", root));
    let base = if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        dir.to_string()
    } else {
        let stem = name.trim_end_matches(".rs");
        if dir.is_empty() {
            stem.to_string()
        } else {
            format!("{dir}/{stem}")
        }
    };
    let rest = if base.is_empty() {
        file
    } else {
        file.strip_prefix(&base)?.strip_prefix('/')?
    };
    let rest = rest.strip_suffix(".rs")?;
    Some(rest.strip_suffix("/mod").unwrap_or(rest).to_string())
}

pub fn parent_path(p: &str) -> &str {
    p.rsplit_once('/').map(|(a, _)| a).unwrap_or("")
}

pub fn is_def(o: &Occurrence) -> bool {
    o.symbol_roles & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_paths() {
        assert_eq!(
            expected_module_path("src/lib.rs", "src/a/b.rs").as_deref(),
            Some("a/b")
        );
        assert_eq!(
            expected_module_path("src/lib.rs", "src/a/mod.rs").as_deref(),
            Some("a")
        );
        assert_eq!(
            expected_module_path("examples/foo.rs", "examples/foo/x.rs").as_deref(),
            Some("x")
        );
        assert_eq!(
            expected_module_path("crates/x/src/main.rs", "src/a.rs"),
            None
        );
    }

    #[test]
    fn module_paths() {
        assert_eq!(
            module_path("rust-analyzer cargo s 0.1.0 crate/").as_deref(),
            Some("")
        );
        assert_eq!(
            module_path("rust-analyzer cargo s 0.1.0 a/b/").as_deref(),
            Some("a/b")
        );
        assert_eq!(module_path("rust-analyzer cargo s 0.1.0 a/B#"), None);
    }

    #[test]
    fn identical_ranges_do_not_form_parent_cycles() {
        let defs = vec![((0, 0, 10, 0), 0), ((2, 0, 5, 1), 1), ((2, 0, 5, 1), 2)];
        let p = parents(&defs, |_| (2, 4));
        assert_eq!(p, vec![(0, None), (1, Some(0)), (2, Some(1))]);
    }

    #[test]
    fn innermost_excludes_self() {
        let defs = vec![((0, 0, 10, 0), 0), ((2, 0, 5, 1), 1), ((3, 4, 3, 9), 2)];
        assert_eq!(innermost(&defs, (3, 5), None), Some(2));
        assert_eq!(innermost(&defs, (3, 5), Some(2)), Some(1));
        assert_eq!(innermost(&defs, (7, 0), None), Some(0));
    }
}
