//! Rust token scanner: identifiers, punctuation, skipping strings, chars and comments.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ident,
    Punct(u8),
    Other,
}

#[derive(Clone, Copy)]
pub struct Tok {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

pub fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// Index just past the closing quote of a (non-raw) string starting at `i` (the opening `"`).
pub fn skip_string(b: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    b.len()
}

/// If a raw string body starts at `i` (`#*"`), return the index past its end.
pub fn skip_raw_string(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    while j < b.len() && b[j] == b'#' {
        j += 1;
    }
    if j >= b.len() || b[j] != b'"' {
        return None;
    }
    let hashes = j - i;
    j += 1;
    while j < b.len() {
        if b[j] == b'"'
            && b[j + 1..]
                .iter()
                .take(hashes)
                .filter(|&&c| c == b'#')
                .count()
                == hashes
        {
            return Some(j + 1 + hashes);
        }
        j += 1;
    }
    Some(b.len())
}

/// Index past a char literal starting at `i` (the opening `'`) or None if it is a lifetime.
pub fn char_literal_end(src: &str, i: usize) -> Option<usize> {
    let b = src.as_bytes();
    if b.get(i + 1) == Some(&b'\\') {
        let mut j = i + 2;
        while j < b.len() {
            match b[j] {
                b'\\' => j += 2,
                b'\'' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return Some(b.len());
    }
    let c = src[i + 1..].chars().next()?;
    let after = i + 1 + c.len_utf8();
    (b.get(after) == Some(&b'\'')).then_some(after + 1)
}

pub fn tokenize(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        match c {
            _ if c.is_ascii_whitespace() => i += 1,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut depth = 1;
                i += 2;
                while i < b.len() && depth > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            b'"' => {
                i = skip_string(b, i);
                toks.push(Tok {
                    kind: Kind::Other,
                    start,
                    end: i,
                });
            }
            b'\'' => {
                if let Some(end) = char_literal_end(src, i) {
                    i = end;
                } else {
                    // Lifetime or label: consume `'ident` so `'use` is not a keyword.
                    i += 1;
                    while i < b.len() && is_ident_byte(b[i]) {
                        i += 1;
                    }
                }
                toks.push(Tok {
                    kind: Kind::Other,
                    start,
                    end: i,
                });
            }
            _ if is_ident_byte(c) => {
                while i < b.len() && is_ident_byte(b[i]) {
                    i += 1;
                }
                let word = &src[start..i];
                let next = b.get(i).copied();
                let mut kind = if c.is_ascii_digit() {
                    Kind::Other
                } else {
                    Kind::Ident
                };
                match (word, next) {
                    ("b" | "c", Some(b'"')) => {
                        i = skip_string(b, i);
                        kind = Kind::Other;
                    }
                    ("b", Some(b'\'')) => {
                        if let Some(end) = char_literal_end(src, i) {
                            i = end;
                            kind = Kind::Other;
                        }
                    }
                    ("r" | "br" | "cr", Some(b'"' | b'#')) => {
                        if let Some(end) = skip_raw_string(b, i) {
                            i = end;
                            kind = Kind::Other;
                        } else if word == "r"
                            && next == Some(b'#')
                            && b.get(i + 1).is_some_and(|&x| is_ident_byte(x))
                        {
                            // raw identifier r#name
                            i += 1;
                            while i < b.len() && is_ident_byte(b[i]) {
                                i += 1;
                            }
                        }
                    }
                    _ => {}
                }
                toks.push(Tok {
                    kind,
                    start,
                    end: i,
                });
            }
            _ => {
                i += 1;
                toks.push(Tok {
                    kind: Kind::Punct(c),
                    start,
                    end: i,
                });
            }
        }
    }
    toks
}

/// Index of the token after the delimiter group opened at `toks[open]`.
pub fn skip_group(toks: &[Tok], open: usize) -> usize {
    let mut depth = 0i32;
    for (j, t) in toks.iter().enumerate().skip(open) {
        match t.kind {
            Kind::Punct(b'(' | b'[' | b'{') => depth += 1,
            Kind::Punct(b')' | b']' | b'}') => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
    }
    toks.len()
}

pub fn is_ident(src: &str, t: Option<&Tok>, word: &str) -> bool {
    t.is_some_and(|t| t.kind == Kind::Ident && &src[t.start..t.end] == word)
}

pub fn is_punct(t: Option<&Tok>, c: u8) -> bool {
    t.is_some_and(|t| t.kind == Kind::Punct(c))
}
