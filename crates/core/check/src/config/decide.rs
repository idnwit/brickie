//! The decisions over a level's rules and links, shared by `check`, the publish gate, the
//! design editor and the viewer:
//!
//! - [`rule_allows`]: whether some rule allows `from -> to` of one kind and scope;
//! - [`judge_link`]: whether a declared link is valid (every kind and scope it names is allowed);
//! - [`covering_link`]: whether a measured edge is allowed (a valid link covers it).

use super::{Brick, Config, KINDS, Link, Rule, Scope, Sel};

/// Whether a rule side matches a brick; `*` never matches an independent brick.
fn matches(sel: &Sel, b: &Brick) -> bool {
    match sel {
        Sel::Any => !b.independent,
        Sel::Tag(t) => b.tags.contains(t),
        Sel::Brick(n) => b.name == *n,
    }
}

/// Edges into an independent brick from a normal one need no rule and no link: it acts like
/// an external library.
pub fn free(from: &Brick, to: &Brick) -> bool {
    to.independent && !from.independent
}

/// The first rule of `level` that allows `from -> to` of `kind` and `scope` (`prod` | `test`).
/// An independent brick may only depend on another independent brick, so nothing allows an
/// edge from it to a normal brick.
pub fn rule_allows<'a>(
    level: &'a Config,
    from: &Brick,
    to: &Brick,
    kind: &str,
    scope: &str,
) -> Option<&'a Rule> {
    if from.independent && !to.independent {
        return None;
    }
    level.rules.iter().find(|r| {
        (r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind))
            && r.scope.matches(scope)
            && matches(&r.from, from)
            && matches(&r.to, to)
    })
}

/// The kinds a list names (empty: all).
pub fn kinds_of(kinds: &[String]) -> Vec<&str> {
    match kinds.is_empty() {
        true => KINDS.to_vec(),
        false => kinds.iter().map(String::as_str).collect(),
    }
}

/// The concrete scopes a scope covers.
fn scopes_of(scope: Scope) -> &'static [&'static str] {
    match scope {
        Scope::Prod => &["prod"],
        Scope::Test => &["test"],
        Scope::All => &["prod", "test"],
    }
}

/// Why `link` of `level` is invalid (`design/link-denied`), or `None` when every kind and
/// scope it names is allowed by a rule. A link into an independent brick needs no rule.
pub fn judge_link(level: &Config, link: &Link) -> Option<String> {
    let (to_name, _) = link.target();
    let (Some(from), Some(to)) = (level.brick(&link.from), level.brick(to_name)) else {
        return Some(format!(
            "`{}` or `{to_name}` is not a brick of this level",
            link.from
        ));
    };
    if from.independent && !to.independent {
        return Some(format!(
            "`{}` is independent and may only link to another independent brick",
            from.name
        ));
    }
    if free(from, to) {
        return None;
    }
    let missing: Vec<String> = kinds_of(&link.kinds)
        .into_iter()
        .flat_map(|k| scopes_of(link.scope).iter().map(move |s| (k, *s)))
        .filter(|(k, s)| rule_allows(level, from, to, k, s).is_none())
        .map(|(k, s)| format!("`{k}` in {s}"))
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "no rule allows `{}` -> `{}` for {}",
            from.name,
            to.name,
            missing.join(", ")
        )
    })
}

/// Why a measured edge is not allowed.
#[derive(Debug, Clone)]
pub enum Why<'a> {
    /// No link covers it.
    Unlinked,
    /// A link covers it, but no rule allows that link (the reason from [`judge_link`]).
    Denied(&'a Link, String),
}

/// The valid link of `level` that covers the sibling edge `from -> to` of `kind` and `scope`,
/// entering `to` through child `entered` (`None` for a leaf). `Ok(None)`: no link is needed
/// (an edge into an independent brick). A link to `composite/child` covers only edges that
/// enter through that child.
pub fn covering_link<'a>(
    level: &'a Config,
    from: &Brick,
    to: &Brick,
    entered: Option<&str>,
    kind: &str,
    scope: &str,
) -> Result<Option<&'a Link>, Why<'a>> {
    if free(from, to) {
        return Ok(None);
    }
    let mut denied = None;
    for l in &level.links {
        let (t, child) = l.target();
        let covers = l.from == from.name
            && t == to.name
            && child.is_none_or(|c| entered == Some(c))
            && (l.kinds.is_empty() || l.kinds.iter().any(|k| k == kind))
            && l.scope.matches(scope);
        if !covers {
            continue;
        }
        match judge_link(level, l) {
            None => return Ok(Some(l)),
            Some(why) => denied = denied.or(Some(Why::Denied(l, why))),
        }
    }
    Err(denied.unwrap_or(Why::Unlinked))
}

/// The links of `level` that sit on a cycle of the level's link graph (brick to brick).
/// Informational only: a code cycle is `deps/cycle`.
pub fn link_cycles(level: &Config) -> Vec<&Link> {
    let reaches = |a: &str, b: &str| {
        let mut seen = vec![a.to_string()];
        let mut todo = vec![a.to_string()];
        while let Some(n) = todo.pop() {
            for l in level.links.iter().filter(|l| l.from == n) {
                let t = l.target().0.to_string();
                if t == b {
                    return true;
                }
                if !seen.contains(&t) {
                    seen.push(t.clone());
                    todo.push(t);
                }
            }
        }
        false
    };
    (level.links.iter())
        .filter(|l| reaches(l.target().0, &l.from))
        .collect()
}
