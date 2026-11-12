//! The path as written at a reference site, recovered from the source text alone.
//!
//! Nothing is resolved: a body reference is the identifier plus the `a::b::` qualifiers directly
//! in front of it (`mx_base::shape::Shape`, or just `double`); a `use` leaf is its full path
//! through enclosing groups (`use a::{b::{c, d}}` gives `a::b::c` for `c`).

use super::Span;
use super::use_scan::tokenizer::{Kind, Tok, is_ident, is_punct, tokenize};

pub struct Written<'a> {
    src: &'a str,
    toks: Vec<Tok>,
    line_starts: Vec<usize>,
}

impl<'a> Written<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0usize];
        line_starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        Self {
            src,
            toks: tokenize(src),
            line_starts,
        }
    }

    fn offset(&self, line: u32, col: u32) -> Option<usize> {
        self.line_starts
            .get(line as usize)
            .map(|s| s + col as usize)
    }

    /// Written path of the reference at `range`; `import` enables use-group prefixes.
    pub fn at(&self, range: Span, import: bool) -> String {
        let leaf = self
            .offset(range.0, range.1)
            .and_then(|o| self.toks.binary_search_by_key(&o, |t| t.start).ok());
        let Some(mut i) = leaf.filter(|&i| self.toks[i].kind == Kind::Ident) else {
            return self.raw(range);
        };
        // `X as Y`: the written path is that of `X`.
        if i >= 2 && is_ident(self.src, self.toks.get(i - 1), "as") {
            i -= 2;
        }
        let text = |i: usize| &self.src[self.toks[i].start..self.toks[i].end];
        let mut parts = vec![text(i)];
        loop {
            let colons = i >= 2
                && is_punct(self.toks.get(i - 1), b':')
                && is_punct(self.toks.get(i - 2), b':')
                && self.toks[i - 2].end == self.toks[i - 1].start;
            let qualifier = if colons {
                i.checked_sub(3)
            } else if import {
                self.group_prefix(i)
            } else {
                None
            };
            match qualifier.filter(|&q| self.toks[q].kind == Kind::Ident) {
                Some(q) => {
                    parts.push(text(q));
                    i = q;
                }
                None => break,
            }
        }
        parts.reverse();
        parts.join("::")
    }

    /// Inside `prefix::{ .. a, b }`: index of the ident before `::{` of the enclosing group.
    fn group_prefix(&self, i: usize) -> Option<usize> {
        let mut depth = 0i32;
        let mut k = i;
        while k > 0 {
            k -= 1;
            match self.toks[k].kind {
                Kind::Punct(b'}') => depth += 1,
                Kind::Punct(b'{') if depth == 0 => {
                    let ok = k >= 3
                        && is_punct(self.toks.get(k - 1), b':')
                        && is_punct(self.toks.get(k - 2), b':');
                    return if ok { Some(k - 3) } else { None };
                }
                Kind::Punct(b'{') => depth -= 1,
                Kind::Punct(b';') => return None,
                _ => {}
            }
        }
        None
    }

    fn raw(&self, r: Span) -> String {
        match (self.offset(r.0, r.1), self.offset(r.2, r.3)) {
            (Some(a), Some(b)) if a <= b && b <= self.src.len() => {
                self.src.get(a..b).unwrap_or_default().to_string()
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str, needle: &str, import: bool) -> String {
        let off = src.find(needle).unwrap();
        let line = src[..off].matches('\n').count() as u32;
        let col = (off - src[..off].rfind('\n').map_or(0, |n| n + 1)) as u32;
        Written::new(src).at((line, col, line, col + 1), import)
    }

    #[test]
    fn body_paths() {
        let s = "fn f() { a::b::run(); x.run(); run(); }";
        assert_eq!(at(s, "run();", false), "a::b::run");
        assert_eq!(at(s, "run(); run", false), "run");
        assert_eq!(at(&s.replace("x.run(); ", ""), "run(); }", false), "run");
    }

    #[test]
    fn use_paths() {
        let s = "pub use a::b::C;\nuse x::{y::{Z, W}, V as U};\n";
        assert_eq!(at(s, "C;", true), "a::b::C");
        assert_eq!(at(s, "W}", true), "x::y::W");
        assert_eq!(at(s, "V as", true), "x::V");
        assert_eq!(at(s, "U}", true), "x::V");
    }
}
