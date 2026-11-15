//! Which attributes mark test code.

/// True for a `#[cfg(...)]` attribute whose predicate is only satisfied under `cfg(test)`: `test`
/// itself, or a direct or nested operand of `all(..)`. `any(..)`, `not(..)` and `feature = ".."`
/// (whatever its name) can hold in production, and `cfg_attr` does not gate an item.
pub fn is_test_cfg(attr: &str) -> bool {
    let t: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t
        .strip_prefix("#!")
        .or_else(|| t.strip_prefix('#'))
        .unwrap_or(&t);
    match t.strip_prefix("[cfg(").and_then(|r| r.strip_suffix(")]")) {
        Some(inner) => requires_test(inner),
        None => false,
    }
}

/// Whether the predicate (whitespace removed) is true only when `test` is set.
fn requires_test(pred: &str) -> bool {
    if pred == "test" {
        return true;
    }
    let Some(args) = pred.strip_prefix("all(").and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    split_top(args).into_iter().any(requires_test)
}

/// Splits on commas outside parentheses and string literals.
fn split_top(s: &str) -> Vec<&str> {
    let (mut out, mut depth, mut in_str, mut start) = (Vec::new(), 0i32, false, 0);
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => depth -= 1,
            ',' if !in_str && depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

pub fn is_test_attr(attr: &str) -> bool {
    let t: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    t.starts_with("#[test") && t.ends_with(']') && (t == "#[test]" || t.starts_with("#[test("))
        || t.ends_with("::test]")
        || t == "#[rstest]"
        || t == "#[bench]"
        || is_test_cfg(attr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_detection() {
        assert!(is_test_cfg("#[cfg(test)]"));
        assert!(is_test_cfg("#[cfg( all( test , feature = \"x\" ) )]"));
        assert!(is_test_cfg("#[cfg(all(feature = \"x\", all(test, y)))]"));
        assert!(!is_test_cfg("#[cfg( any( test , feature = \"x\" ) )]"));
        assert!(!is_test_cfg("#[cfg(any(test, unix))]"));
        assert!(!is_test_cfg("#[cfg(all(unix, any(test, windows)))]"));
        assert!(!is_test_cfg("#[cfg(all(unix, not(test)))]"));
        assert!(!is_test_cfg("#[cfg_attr(test, derive(Debug))]"));
        assert!(!is_test_cfg("#[cfg(all(feature = \"a,test\", unix))]"));
        assert!(is_test_cfg("#[cfg(all(test, unix))]"));
        assert!(!is_test_cfg("#[cfg(feature = \"test-util\")]"));
        assert!(is_test_cfg("#![cfg(test)]"));
        assert!(!is_test_cfg("#[cfg(not(test))]"));
        assert!(!is_test_cfg("#[cfg(any(unix, not(test)))]"));
        assert!(!is_test_cfg("#[cfg(feature = \"test\")]"));
        assert!(!is_test_cfg("#[cfg(feature = \"testing\")]"));
        assert!(!is_test_cfg("#[derive(Debug)]"));
    }
}
