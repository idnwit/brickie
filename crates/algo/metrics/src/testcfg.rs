//! Which attributes mark test code.

/// True for a `#[cfg(...)]` attribute that enables test code: `cfg(test)`, `cfg(any(test, ..))`,
/// `cfg(all(test, ..))` and `feature = "test-util"` gates. Anything with `not(..)` is not test.
pub fn is_test_cfg(attr: &str) -> bool {
    let t: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t
        .strip_prefix("#!")
        .or_else(|| t.strip_prefix('#'))
        .unwrap_or(&t);
    let Some(inner) = t.strip_prefix("[cfg(").and_then(|r| r.strip_suffix(")]")) else {
        return false;
    };
    if inner.contains("not(") {
        return false;
    }
    if inner.contains("feature=\"test-util\"") {
        return true;
    }
    // Identifier tokens outside string literals: a bare `test` enables test code.
    let mut in_str = false;
    let mut tok = String::new();
    for c in inner.chars().chain(std::iter::once(',')) {
        if c == '"' {
            in_str = !in_str;
            tok.clear();
        } else if in_str {
        } else if c.is_alphanumeric() || c == '_' || c == '-' {
            tok.push(c);
        } else {
            if tok == "test" {
                return true;
            }
            tok.clear();
        }
    }
    false
}

pub fn is_test_attr(attr: &str) -> bool {
    let t: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    t.starts_with("#[test") && t.ends_with(']') && (t == "#[test]" || t.starts_with("#[test("))
        || t.ends_with("::test]")
        || t == "#[rstest]"
        || t == "#[bench]"
        || is_test_cfg(attr)
}
