//! `// brickie: allow(code, ...) -- reason` comment directives and the item line each one sits
//! above. Stable Rust has no custom tool attributes, so the directive is a line comment.

/// One directive comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    /// 1-based line of the comment.
    pub line: u32,
    /// 1-based line of the code the directive sits above (doc comments and attributes between
    /// the two are skipped); `None` when a blank line, the end of the file or another comment
    /// breaks the link.
    pub target: Option<u32>,
    /// The codes listed in `allow(...)`, trimmed, in order.
    pub codes: Vec<String>,
    /// The text after ` -- `, trimmed; empty when missing.
    pub reason: String,
}

/// Parses one line: `// brickie: allow(a, b) -- why`. Doc comments never count.
fn parse_line(line: &str) -> Option<(Vec<String>, String)> {
    let rest = line.trim_start().strip_prefix("//")?;
    if rest.starts_with('/') || rest.starts_with('!') {
        return None;
    }
    let rest = rest.trim_start().strip_prefix("brickie:")?;
    let rest = rest.trim_start().strip_prefix("allow(")?;
    let (list, tail) = rest.split_once(')')?;
    let codes = list
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(String::from)
        .collect();
    let reason = tail
        .trim_start()
        .strip_prefix("--")
        .map(|r| r.trim().to_string())
        .unwrap_or_default();
    Some((codes, reason))
}

/// Net `[` minus `]` of a line.
fn bracket_delta(line: &str) -> i32 {
    line.chars().fold(0, |d, c| match c {
        '[' => d + 1,
        ']' => d - 1,
        _ => d,
    })
}

/// The first line after `from` (0-based) that is code: doc comments (`///`, `/** */`) and
/// attributes (`#[...]`, possibly over several lines) are skipped.
fn target(lines: &[&str], from: usize) -> Option<u32> {
    let mut i = from;
    while let Some(l) = lines.get(i) {
        let t = l.trim();
        if t.is_empty() {
            return None;
        }
        if t.starts_with("///") {
            i += 1;
        } else if t.starts_with("/**") && !t.starts_with("/**/") {
            while lines.get(i).is_some_and(|l| !l.contains("*/")) {
                i += 1;
            }
            i += 1;
        } else if t.starts_with("#[") {
            let mut depth = 0;
            while let Some(l) = lines.get(i) {
                depth += bracket_delta(l);
                i += 1;
                if depth <= 0 {
                    break;
                }
            }
        } else if t.starts_with("//") {
            return None;
        } else {
            return Some(i as u32 + 1);
        }
    }
    None
}

/// Every directive comment of `src`, in file order.
pub fn directives(src: &str) -> Vec<Directive> {
    let lines: Vec<&str> = src.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let (codes, reason) = parse_line(l)?;
            Some(Directive {
                line: i as u32 + 1,
                target: target(&lines, i + 1),
                codes,
                reason,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> Directive {
        let d = directives(src);
        assert_eq!(d.len(), 1, "{d:?}");
        d.into_iter().next().unwrap()
    }

    #[test]
    fn directly_above_item() {
        let d = one(
            "use a;\n// brickie: allow(interface/unused) -- build.rs picks it\npub struct A;\n",
        );
        assert_eq!(d.line, 2);
        assert_eq!(d.target, Some(3));
        assert_eq!(d.codes, ["interface/unused"]);
        assert_eq!(d.reason, "build.rs picks it");
    }

    #[test]
    fn through_docs_and_attributes() {
        let src = "// brickie: allow(interface/unused) -- r\n/// Doc.\n/** More\n docs */\n#[derive(\n    Debug,\n)]\n#[inline]\npub fn f() {}\n";
        assert_eq!(one(src).target, Some(9));
    }

    #[test]
    fn broken_by_blank_line_or_code_comment() {
        assert_eq!(
            one("// brickie: allow(x) -- r\n\npub fn f() {}\n").target,
            None
        );
        assert_eq!(
            one("// brickie: allow(x) -- r\n// note\npub fn f() {}\n").target,
            None
        );
        assert_eq!(one("// brickie: allow(x) -- r\n").target, None);
    }

    #[test]
    fn reason_and_codes() {
        let d = one("// brickie: allow(a/b, c/d) -- two\nfn f() {}\n");
        assert_eq!(d.codes, ["a/b", "c/d"]);
        let d = one("// brickie: allow(a/b)\nfn f() {}\n");
        assert_eq!(d.reason, "");
        let d = one("// brickie: allow(a/b) --   \nfn f() {}\n");
        assert_eq!(d.reason, "");
    }

    #[test]
    fn ignores_docs_and_other_comments() {
        assert!(directives("/// brickie: allow(a/b) -- r\nfn f() {}\n").is_empty());
        assert!(directives("// brickie: deny(a/b)\n// other\nfn f() {}\n").is_empty());
    }
}
