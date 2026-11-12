//! Small pure helpers over SCIP symbols and ranges.

use super::Span;

/// Descriptor names of a SCIP symbol without the `crate` segment; `None` when it does not parse.
pub fn descriptor_names(scip: &str) -> Option<Vec<String>> {
    let sym = scip::symbol::parse_symbol(scip).ok()?;
    Some(
        sym.descriptors
            .into_iter()
            .map(|d| d.name)
            .filter(|n| n != "crate")
            .collect(),
    )
}

pub fn module_path(scip: &str) -> Option<String> {
    if !scip.ends_with('/') {
        return None;
    }
    Some(descriptor_names(scip)?.join("/"))
}

/// A rust-analyzer local (`local N`), which has no stable name across documents.
pub fn is_local(scip: &str) -> bool {
    scip.starts_with("local")
}

/// Innermost definition (by enclosing range) containing `pos`, excluding
/// `exclude`. For a def occurrence the result can be a child (a module def at (0,0) shares its
/// range with its first child); edges and reach drop def occurrences, so that container never
/// reaches output. `defs` are (body span, symbol index) sorted by start asc, end desc.
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

/// SCIP `SymbolRole::Definition` bit in an occurrence's roles.
pub fn is_def(roles: i32) -> bool {
    roles & 1 == 1
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
    fn descriptor_names_drop_crate_and_reject_garbage() {
        assert_eq!(
            descriptor_names("rust-analyzer cargo s 0.1.0 crate/a/B#"),
            Some(vec!["a".to_string(), "B".to_string()])
        );
        assert_eq!(
            descriptor_names("rust-analyzer cargo s 0.1.0 crate/"),
            Some(vec![])
        );
        assert_eq!(descriptor_names("not a symbol"), None);
    }

    #[test]
    fn local_and_definition_checks() {
        assert!(is_local("local 3") && !is_local("rust-analyzer cargo s 0.1.0 a/"));
        assert!(is_def(1) && is_def(3) && !is_def(0) && !is_def(2));
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
