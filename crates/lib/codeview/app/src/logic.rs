//! The viewer's pure logic: URL params, explorer filtering, quick open matching and tabs.

use codeview_port::Node;

/// The viewer's URL params: `path`, `line`, `col` (a byte column), `view` and `filter` (a
/// node id).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub path: Option<String>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub view: Option<String>,
    pub filter: Option<String>,
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// Percent-decode a query value (`+` is a space).
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let pair = (b[i] == b'%')
            .then(|| Some(hex(*b.get(i + 1)?)? * 16 + hex(*b.get(i + 2)?)?))
            .flatten();
        match (pair, b[i]) {
            (Some(v), _) => {
                out.push(v);
                i += 2;
            }
            (None, b'+') => out.push(b' '),
            (None, c) => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl Query {
    /// Parse `?path=a.rs&line=3` (the leading `?` is optional).
    pub fn parse(search: &str) -> Query {
        let mut q = Query::default();
        for pair in search.trim_start_matches('?').split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = decode(v);
            match k {
                "path" if !v.is_empty() => q.path = Some(v),
                "line" => q.line = v.parse().ok(),
                "col" => q.col = v.parse().ok(),
                "view" if !v.is_empty() => q.view = Some(v),
                "filter" if !v.is_empty() => q.filter = Some(v),
                _ => {}
            }
        }
        q
    }
}

/// The node with id `id` in `roots`.
fn find<'a>(roots: &'a [Node], id: &str) -> Option<&'a Node> {
    roots.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

/// The roots shown under `filter`: the filtered node alone, or everything when it is unknown.
pub fn scoped(roots: &[Node], filter: Option<&str>) -> Vec<Node> {
    match filter.and_then(|f| find(roots, f)) {
        Some(n) => vec![n.clone()],
        None => roots.to_vec(),
    }
}

/// Every file path of a tree, depth first, without duplicates.
pub fn files(roots: &[Node]) -> Vec<String> {
    fn walk(n: &Node, out: &mut Vec<String>) {
        if let Some(p) = &n.path
            && !out.contains(p)
        {
            out.push(p.clone());
        }
        n.children.iter().for_each(|c| walk(c, out));
    }
    let mut out = Vec::new();
    roots.iter().for_each(|n| walk(n, &mut out));
    out
}

/// The file to open first for a scoped tree (`files` order): a crate or folder root
/// (`lib.rs`, `main.rs`, `mod.rs`) at the shallowest depth, else the first file.
pub fn entry_file(files: &[String]) -> Option<String> {
    let root = |p: &&String| matches!(file_name(p), "lib.rs" | "main.rs" | "mod.rs");
    files
        .iter()
        .filter(root)
        .min_by_key(|p| p.matches('/').count())
        .or(files.first())
        .cloned()
}

/// A stable hue (0..360) for a tag, so the same tag always gets the same colour.
pub fn tag_hue(tag: &str) -> u32 {
    let h = tag.bytes().fold(2166136261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16777619)
    });
    h % 360
}

/// Quick open: the paths containing the characters of `query` in order (case-insensitive),
/// best first: a match in the file name, then the shorter span, then the shorter path.
pub fn quick(query: &str, paths: &[String], limit: usize) -> Vec<String> {
    let q: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut hits: Vec<(bool, usize, usize, &String)> = paths
        .iter()
        .filter_map(|p| {
            let lower = p.to_lowercase();
            let (start, end) = subsequence(&lower, &q)?;
            let name_at = lower.rfind('/').map_or(0, |i| i + 1);
            Some((start < name_at, end - start, p.len(), p))
        })
        .collect();
    hits.sort();
    hits.into_iter().take(limit).map(|h| h.3.clone()).collect()
}

/// Char span `(first, last + 1)` of the leftmost-ending match of `q` in `s`, as byte offsets.
fn subsequence(s: &str, q: &[char]) -> Option<(usize, usize)> {
    if q.is_empty() {
        return Some((0, 0));
    }
    let mut it = q.iter();
    let mut want = it.next();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if Some(&c) == want {
            start.get_or_insert(i);
            want = it.next();
            if want.is_none() {
                return Some((start?, i + c.len_utf8()));
            }
        }
    }
    None
}

/// The tabs after opening `path` (appended when new).
pub fn open_tab(tabs: &[String], path: &str) -> Vec<String> {
    let mut t = tabs.to_vec();
    if !t.iter().any(|p| p == path) {
        t.push(path.to_string());
    }
    t
}

/// The tabs after closing `path`, and the tab to show next when `path` was active.
pub fn close_tab(tabs: &[String], path: &str) -> (Vec<String>, Option<String>) {
    let i = tabs.iter().position(|p| p == path);
    let t: Vec<String> = tabs.iter().filter(|p| *p != path).cloned().collect();
    let next = i.and_then(|i| t.get(i.min(t.len().saturating_sub(1))).cloned());
    (t, next)
}

/// The last component of a path.
pub fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
#[path = "logic_tests.rs"]
mod tests;
