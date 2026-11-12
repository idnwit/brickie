//! Render params: the whole UI state that lives in the URL.

use serde::{Deserialize, Serialize};

/// Render params: the whole UI state that lives in the URL. Empty means "default".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Params {
    /// Brick name filter (diagnostics and functions).
    pub brick: String,
    /// `error` | `warn`.
    pub level: String,
    /// `open` | `accepted`.
    pub status: String,
    /// Diagnostic code, exact (`deps/deny`) or a family prefix (`complexity`).
    pub code: String,
    /// Function metric: cognitive (default), cyclomatic, nesting, paths, params, lines, name.
    pub fn_sort: String,
    /// `desc` (default) | `asc`.
    pub fn_dir: String,
    /// `prod` (default) | `test` | `all`.
    pub fn_scope: String,
    /// Row limit: a number (default 50) or `all`.
    pub fn_limit: String,
    /// View preset: `arch` (default) | `viol` | `all` | `custom`.
    pub p: String,
    /// Data-layer differences from the preset, `-cx,+tests`.
    pub lay: String,
    /// Violations scope: `all` (default) | `deps` | `cx` | `stale`.
    pub vs: String,
    /// Findings group filter: `deps` | `cx` | `stale` (empty = all).
    pub grp: String,
    /// Selected tag rows (group selection), comma separated.
    pub group: String,
    /// Selected edge `from>to>kind`.
    pub edge: String,
    /// Selected finding id.
    pub item: String,
    /// Current level of the brick tree: `root` (default), `root/web`.
    pub lv: String,
    /// `1`: the findings list shows only findings of this level and below.
    pub below: String,
    /// Edges from outside neighbours inside a composite: `children` (default) | `frame`.
    pub bnd: String,
    /// `1`: show the open change's preview and the editor (empty: the current design).
    pub change: String,
}

pub const DEFAULT_LIMIT: usize = 50;

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Percent-encoded query value of `s` (for building code viewer links).
pub fn encode(s: &str) -> String {
    enc(s)
}

impl Params {
    /// The selected bricks of `brick` (comma separated names).
    pub fn bricks(&self) -> Vec<&str> {
        self.brick.split(',').filter(|b| !b.is_empty()).collect()
    }
    /// The same view at level `lv`: the brick, group, edge and finding selections are names of
    /// the old level, so they are cleared.
    pub fn at_level(&self, lv: &str) -> Params {
        let mut p = self.clone();
        p.lv = if lv == "root" {
            String::new()
        } else {
            lv.to_string()
        };
        p.brick.clear();
        p.group.clear();
        p.edge.clear();
        p.item.clear();
        p
    }

    /// The parent level of the current one (`None` at the root).
    pub fn parent_level(&self) -> Option<&str> {
        self.level_path().rsplit_once('/').map(|(a, _)| a)
    }

    /// The current level path (`root` by default).
    pub fn level_path(&self) -> &str {
        if self.lv.is_empty() { "root" } else { &self.lv }
    }
    /// The change preview (and the editor) is on.
    pub fn editing(&self) -> bool {
        self.change == "1"
    }
    /// Outside edges end at the frame instead of the children.
    pub fn to_frame(&self) -> bool {
        self.bnd == "frame"
    }
    pub fn preset(&self) -> &str {
        if self.p.is_empty() { "arch" } else { &self.p }
    }
    pub fn vscope(&self) -> &str {
        if self.vs.is_empty() { "all" } else { &self.vs }
    }
    pub fn sort(&self) -> &str {
        if self.fn_sort.is_empty() {
            "cognitive"
        } else {
            &self.fn_sort
        }
    }
    pub fn desc(&self) -> bool {
        self.fn_dir != "asc"
    }
    pub fn scope(&self) -> &str {
        if self.fn_scope.is_empty() {
            "prod"
        } else {
            &self.fn_scope
        }
    }
    pub fn limit(&self) -> Option<usize> {
        match self.fn_limit.as_str() {
            "all" => None,
            s => Some(s.parse().unwrap_or(DEFAULT_LIMIT)),
        }
    }

    /// `a=b&c=d` with defaults omitted, in a fixed order, so equal views have equal URLs.
    pub fn query(&self) -> String {
        let pairs = [
            ("brick", self.brick.as_str(), ""),
            ("level", &self.level, ""),
            ("status", &self.status, ""),
            ("code", &self.code, ""),
            ("fn_sort", &self.fn_sort, "cognitive"),
            ("fn_dir", &self.fn_dir, "desc"),
            ("fn_scope", &self.fn_scope, "prod"),
            ("fn_limit", &self.fn_limit, "50"),
            ("p", &self.p, "arch"),
            ("lay", &self.lay, ""),
            ("vs", &self.vs, "all"),
            ("grp", &self.grp, ""),
            ("group", &self.group, ""),
            ("edge", &self.edge, ""),
            ("item", &self.item, ""),
            ("lv", &self.lv, "root"),
            ("below", &self.below, ""),
            ("bnd", &self.bnd, "children"),
            ("change", &self.change, ""),
        ];
        pairs
            .iter()
            .filter(|(_, v, d)| !v.is_empty() && v != d)
            .map(|(k, v, _)| format!("{k}={}", enc(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    pub fn url(&self) -> String {
        match self.query() {
            q if q.is_empty() => "/".to_string(),
            q => format!("/?{q}"),
        }
    }

    /// Parse a query string (`?a=b&c=d`, leading `?` optional).
    pub fn from_query(q: &str) -> Params {
        let mut p = Params::default();
        for kv in q.trim_start_matches('?').split('&') {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            let v = decode(v);
            match k {
                "brick" => p.brick = v,
                "lv" => p.lv = v,
                "below" => p.below = v,
                "bnd" => p.bnd = v,
                "change" => p.change = v,
                "level" => p.level = v,
                "status" => p.status = v,
                "code" => p.code = v,
                "fn_sort" => p.fn_sort = v,
                "fn_dir" => p.fn_dir = v,
                "fn_scope" => p.fn_scope = v,
                "fn_limit" => p.fn_limit = v,
                "p" => p.p = v,
                "lay" => p.lay = v,
                "vs" => p.vs = v,
                "grp" => p.grp = v,
                "group" => p.group = v,
                "edge" => p.edge = v,
                "item" => p.item = v,
                _ => {}
            }
        }
        p
    }

    /// Copy with one change applied.
    pub fn with(&self, f: impl FnOnce(&mut Params)) -> Params {
        let mut p = self.clone();
        f(&mut p);
        p
    }
}

/// Percent-decode a query value.
pub(crate) fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' => {
                let hex = b
                    .get(i + 1..i + 3)
                    .and_then(|h| std::str::from_utf8(h).ok())
                    .and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_round_trip_and_omit_defaults() {
        let p = Params::from_query("?brick=engine&level=error&code=deps%2Fdeny&fn_sort=cognitive");
        assert_eq!(p.brick, "engine");
        assert_eq!(p.code, "deps/deny");
        assert_eq!(p.query(), "brick=engine&level=error&code=deps/deny");
        assert_eq!(
            Params::from_query(&p.query()),
            p.with(|p| p.fn_sort.clear())
        );
        assert_eq!(Params::default().url(), "/");
    }
}
