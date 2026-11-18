//! A small Rust lexer for the docs page: source in, HTML out with `<span class="h-..">` around
//! keywords, strings, comments, numbers, lifetimes, macros, types and attributes.

use super::text::esc;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while",
];

fn span(o: &mut String, class: &str, s: &str) {
    o.push_str("<span class=\"h-");
    o.push_str(class);
    o.push_str("\">");
    o.push_str(&esc(s));
    o.push_str("</span>");
}

fn ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn ident_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Length in bytes of the string literal starting at `s[0]` (`"`, `r#"`, `b"`, `br"`...), if any.
fn string_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix('b').unwrap_or(s);
    let pre = s.len() - rest.len();
    if let Some(body) = rest.strip_prefix('"') {
        let mut esc = false;
        for (i, c) in body.char_indices() {
            match c {
                _ if esc => esc = false,
                '\\' => esc = true,
                '"' => return Some(pre + 1 + i + 1),
                _ => {}
            }
        }
        return Some(s.len());
    }
    let body = rest.strip_prefix('r')?;
    let hashes = body.chars().take_while(|&c| c == '#').count();
    let body = body[hashes..].strip_prefix('"')?;
    let end = format!("\"{}", "#".repeat(hashes));
    let used = pre + 1 + hashes + 1;
    Some(body.find(&end).map_or(s.len(), |i| used + i + end.len()))
}

/// Length of the char literal at `s[0]` (`'`), or `None` when it is a lifetime.
fn char_len(s: &str) -> Option<usize> {
    let body = &s[1..];
    let mut it = body.char_indices();
    let (_, c) = it.next()?;
    if c == '\\' {
        let end = body.find('\'').filter(|&i| i > 1)?;
        return Some(end + 2);
    }
    let (i, q) = it.next()?;
    (q == '\'').then_some(1 + i + 1)
}

/// Length of the attribute `#[...]` or `#![...]` at `s[0]`.
fn attr_len(s: &str) -> usize {
    let mut depth = 0;
    for (j, ch) in s.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
    }
    s.len()
}

/// Length of the number at `s[0]`; `1..2` is a range, not a float.
fn number_len(s: &str) -> usize {
    let n = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(s.len());
    s[..n].find("..").unwrap_or(n)
}

/// Class and length of the word at `s[0]`: a macro name with its `!`, keyword, type or plain.
fn word(s: &str) -> (Option<&'static str>, usize) {
    let n = s.find(|c| !ident_char(c)).unwrap_or(s.len());
    let w = &s[..n];
    if s[n..].starts_with('!') && !s[n..].starts_with("!=") {
        (Some("m"), n + 1)
    } else if KEYWORDS.contains(&w) {
        (Some("k"), n)
    } else if w.starts_with(char::is_uppercase) && w.chars().any(char::is_lowercase) {
        (Some("t"), n)
    } else {
        (None, n)
    }
}

/// Char literal or lifetime at `s[0]` (`'`).
fn quote(s: &str) -> (&'static str, usize) {
    match char_len(s) {
        Some(n) => ("s", n),
        None => (
            "l",
            1 + s[1..].find(|c| !ident_char(c)).unwrap_or(s.len() - 1),
        ),
    }
}

/// Class (`None` for plain text) and byte length of the token at the start of `s`.
fn token(s: &str) -> (Option<&'static str>, usize) {
    let c = s.chars().next().unwrap_or(' ');
    if s.starts_with("//") {
        (Some("c"), s.find('\n').unwrap_or(s.len()))
    } else if s.starts_with("/*") {
        (Some("c"), s.find("*/").map_or(s.len(), |e| e + 2))
    } else if let Some(n) = s.strip_prefix("b'").and_then(|_| char_len(&s[1..])) {
        (Some("s"), n + 1)
    } else if let Some(n) = string_len(s).filter(|_| !s.starts_with("b'")) {
        (Some("s"), n)
    } else if c == '\'' {
        let (k, n) = quote(s);
        (Some(k), n)
    } else if s.starts_with("#[") || s.starts_with("#![") {
        (Some("a"), attr_len(s))
    } else if c.is_ascii_digit() {
        (Some("n"), number_len(s))
    } else if ident_start(c) {
        word(s)
    } else {
        (None, c.len_utf8())
    }
}

/// `src` highlighted as Rust, HTML-escaped.
pub fn rust(src: &str) -> String {
    let mut o = String::with_capacity(src.len() * 2);
    let mut i = 0;
    while i < src.len() {
        let (class, n) = token(&src[i..]);
        match class {
            Some(k) => span(&mut o, k, &src[i..i + n]),
            None => o.push_str(&esc(&src[i..i + n])),
        }
        i += n;
    }
    o
}

#[cfg(test)]
mod tests {
    use super::rust;

    #[test]
    fn highlights_tokens() {
        let h = rust("pub fn f<'a>(x: &'a Vec<u8>) { println!(\"a<b\"); } // hi\n#[derive(Debug)]");
        for want in [
            "<span class=\"h-k\">pub</span>",
            "<span class=\"h-l\">&#39;a</span>",
            "<span class=\"h-t\">Vec</span>",
            "<span class=\"h-m\">println!</span>",
            "<span class=\"h-s\">&quot;a&lt;b&quot;</span>",
            "<span class=\"h-c\">// hi</span>",
            "<span class=\"h-a\">#[derive(Debug)]</span>",
        ] {
            assert!(h.contains(want), "{want} in {h}");
        }
    }

    #[test]
    fn literals_and_escaping() {
        let h = rust("let a = r#\"x\"y\"#; let b = 'c'; let c = b'\\n'; 1..2 0x1f_u8 a != b");
        assert!(
            h.contains("<span class=\"h-s\">r#&quot;x&quot;y&quot;#</span>"),
            "{h}"
        );
        assert!(h.contains("<span class=\"h-s\">&#39;c&#39;</span>"), "{h}");
        assert!(
            h.contains("<span class=\"h-s\">b&#39;\\n&#39;</span>"),
            "{h}"
        );
        assert!(
            h.contains("<span class=\"h-n\">1</span>..<span class=\"h-n\">2</span>"),
            "{h}"
        );
        assert!(h.contains("<span class=\"h-n\">0x1f_u8</span>"), "{h}");
        assert!(!h.contains("h-m"), "{h}");
        assert_eq!(rust("<b>&"), "&lt;b&gt;&amp;");
    }
}
