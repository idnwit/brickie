//! Diagnostics as the webview receives them (the CLI's `Diagnostic` field for field), the cache
//! header.

use super::{Params, encode};
use serde::{Deserialize, Serialize};

/// The measure cache header: which workspace and commit it describes and how fresh it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheInfo {
    pub workspace: String,
    pub commit: Option<String>,
    pub dirty: bool,
    /// UTC time the cache file was written.
    pub measured: String,
    /// Seconds since the cache file was written, when the page was built (`None`: unknown).
    #[serde(default)]
    pub age_secs: Option<u64>,
    /// Files changed since the measure (`stale/file` findings).
    #[serde(default)]
    pub stale_files: usize,
}

/// A source location: file, 1-based line, and optionally a byte column and length to mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanView {
    pub file: String,
    pub line: u32,
    pub col: Option<u32>,
    pub len: Option<u32>,
    pub label: Option<String>,
}

impl SpanView {
    /// `file:line:col`, `file:line` or just `file` (line 0 = whole file).
    pub fn loc(&self) -> String {
        match (self.line, self.col) {
            (0, _) => self.file.clone(),
            (l, None) => format!("{}:{l}", self.file),
            (l, Some(c)) => format!("{}:{l}:{c}", self.file),
        }
    }

    /// The code viewer at this span.
    pub fn href(&self) -> String {
        code_href(&self.file, self.line, self.col)
    }
}

/// The code viewer at a file, line and byte column (`/code/?path=&line=&col=`); line 0 opens
/// the file without moving.
pub fn code_href(file: &str, line: u32, col: Option<u32>) -> String {
    let mut s = format!("{}?path={}&line={line}", crate::CODE_PATH, encode(file));
    if let Some(c) = col {
        s += &format!("&col={c}");
    }
    s
}

/// The code viewer with the Bricks explorer scoped to the brick `node` (its level path, for
/// example `root/web/server`); the viewer opens the brick's entry file.
pub fn code_brick_href(node: &str) -> String {
    format!("{}?filter={}&view=Bricks", crate::CODE_PATH, encode(node))
}

/// A note of a finding: text with an optional location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteView {
    pub text: String,
    pub span: Option<SpanView>,
}

/// The rule a finding came from and where it is declared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleView {
    pub name: String,
    /// `file:line` in the config.
    pub loc: Option<String>,
}

/// One source line with the primary span cut out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub line: u32,
    pub before: String,
    pub hit: String,
    pub after: String,
}

/// The same fields as the CLI's `Diagnostic` (plus a snippet the CLI prints from disk).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagView {
    pub id: String,
    pub code: String,
    pub message: String,
    pub span: Option<SpanView>,
    pub spans: Vec<SpanView>,
    pub rule: Option<RuleView>,
    pub help: Option<String>,
    pub notes: Vec<NoteView>,
    pub bricks: Vec<String>,
    /// The level whose children `bricks` names (`root`, `root/web`); none for non-brick findings.
    #[serde(default)]
    pub brick_level: Option<String>,
    pub snippet: Option<Snippet>,
}

/// A diagnostic code filter: empty, exact (`deps/unlinked`) or a family prefix (`complexity`).
pub fn code_matches(code: &str, filter: &str) -> bool {
    filter.is_empty()
        || code == filter
        || code
            .strip_prefix(filter)
            .is_some_and(|r| r.starts_with('/'))
}

/// Group and sub-layer key of a diagnostic code.
pub fn group_sub(code: &str) -> (&'static str, String) {
    if let Some(r) = code.strip_prefix("deps/") {
        let sub = match r {
            "deny" => "d-deny",
            "default" => "d-default",
            "private" => "d-private",
            "cycle" => "d-cycle",
            _ => return ("deps", "deps".into()),
        };
        return ("deps", sub.into());
    }
    if let Some(m) = code.strip_prefix("complexity/") {
        return ("cx", format!("c-{m}"));
    }
    if code.starts_with("interface/") {
        return ("iface", "i-unused".into());
    }
    let sub = match code {
        "stale/file" => "s-file",
        "bricks/unmapped" => "s-unmapped",
        _ => "s-other",
    };
    ("stale", sub.into())
}

/// Whether a diagnostic passes the filters (without graph-based group expansion).
pub fn diag_visible(d: &DiagView, p: &Params) -> bool {
    diag_visible_in(d, p, &super::GraphView::default())
}

/// Whether a diagnostic passes the filters in the context of `g`: `?brick=<group>` shows findings
/// for the group and all its members while `?brick=<member>` shows that member's findings (plus
/// `deps/cycle` findings on its enclosing group).
pub fn diag_visible_in(d: &DiagView, p: &Params, g: &super::GraphView) -> bool {
    code_matches(&d.code, &p.code)
        && (p.grp.is_empty() || group_sub(&d.code).0 == p.grp)
        && (p.brick.is_empty() || p.bricks().iter().any(|sel| diag_matches_sel(d, sel, g)))
}

fn diag_matches_sel(d: &DiagView, sel: &str, g: &super::GraphView) -> bool {
    if let Some(grp) = g
        .bricks
        .iter()
        .find(|b| b.is_group && (b.name == sel || b.path == sel))
    {
        return super::diag_hits(d, &grp.path, &grp.name)
            || grp
                .group_members
                .iter()
                .any(|m| super::diag_hits(d, &m.path, &m.name));
    }
    if let Some((grp, m)) = g.bricks.iter().find_map(|b| {
        b.group_members
            .iter()
            .find(|m| m.name == sel || m.path == sel)
            .map(|m| (b, m))
    }) {
        return super::diag_hits_member(d, &m.path, &m.name, &grp.path, &grp.name);
    }
    if let Some(b) = g.bricks.iter().find(|b| b.name == sel || b.path == sel) {
        return super::diag_hits(d, &b.path, &b.name);
    }
    super::diag_hits(d, "", sel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_links_open_the_code_viewer() {
        assert_eq!(
            code_href("crates/a b.rs", 12, Some(5)),
            "/code/?path=crates/a%20b.rs&line=12&col=5"
        );
        assert_eq!(code_href("a.rs", 3, None), "/code/?path=a.rs&line=3");
    }

    #[test]
    fn diag_visible_in_expands_group_but_keeps_member_scoped() {
        let g = super::super::GraphView {
            bricks: vec![super::super::BrickView {
                name: "services".into(),
                path: "root/services".into(),
                is_group: true,
                group_members: vec![
                    super::super::GroupMemberView {
                        name: "auth".into(),
                        path: "root/auth".into(),
                        ..Default::default()
                    },
                    super::super::GroupMemberView {
                        name: "billing".into(),
                        path: "root/billing".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
        let d_auth = DiagView {
            id: "1".into(),
            code: "complexity/cognitive".into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: vec!["auth".into()],
            brick_level: Some("root".into()),
            snippet: None,
        };
        let d_bill = DiagView {
            id: "2".into(),
            bricks: vec!["billing".into()],
            ..d_auth.clone()
        };
        let p_grp = Params::from_query("brick=services");
        assert!(diag_visible_in(&d_auth, &p_grp, &g));
        assert!(diag_visible_in(&d_bill, &p_grp, &g));
        let p_mem = Params::from_query("brick=auth");
        assert!(diag_visible_in(&d_auth, &p_mem, &g));
        assert!(!diag_visible_in(&d_bill, &p_mem, &g));

        // Enclosing-group deps/cycle matches member selection, while other group findings do not.
        let d_cycle = DiagView {
            id: "3".into(),
            code: "deps/cycle".into(),
            bricks: vec!["services".into()],
            ..d_auth.clone()
        };
        let d_grp_other = DiagView {
            id: "4".into(),
            code: "design/planned".into(),
            bricks: vec!["services".into()],
            ..d_auth.clone()
        };
        assert!(diag_visible_in(&d_cycle, &p_mem, &g));
        assert!(!diag_visible_in(&d_grp_other, &p_mem, &g));

        // Same-named brick in a different composite must not collide.
        let d_other_level = DiagView {
            id: "5".into(),
            brick_level: Some("root/other".into()),
            ..d_auth.clone()
        };
        assert!(!diag_visible_in(&d_other_level, &p_mem, &g));
        assert!(!diag_visible_in(&d_other_level, &p_grp, &g));
    }
}
