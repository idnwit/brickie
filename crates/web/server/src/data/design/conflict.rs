//! Whether a stale write may be re-sent on the new head: only when no version between its base
//! and the head touched an identity the write touches (a brick by name, a rule by file and
//! key, a complexity limit by file and `bricks`). A version without ops (a hand edit or an
//! undo) is unknown and touches everything.

use brickie_change::{Op, RuleKey, RuleSpec, Version};

/// A thing an op touches.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ident {
    Brick(String),
    /// A brick a rule names: clashes with changes to the brick, not with other rules naming it.
    Named(String),
    Rule(String, RuleKey),
    Limit(String, String),
    /// File-level settings: exports, deps defaults, includes, the file itself.
    File(String),
}

impl Ident {
    /// Two identities clash when they name the same thing. A rule key without `kinds` or
    /// `scope` stands for any narrowing.
    fn clashes(&self, other: &Ident) -> bool {
        match (self, other) {
            (Ident::Rule(fa, a), Ident::Rule(fb, b)) => {
                fa == fb
                    && a.from == b.from
                    && a.to == b.to
                    && same_set(&a.kinds, &b.kinds)
                    && match (&a.scope, &b.scope) {
                        (Some(x), Some(y)) => x == y,
                        _ => true,
                    }
            }
            (Ident::Named(a), Ident::Brick(b)) | (Ident::Brick(a), Ident::Named(b)) => a == b,
            (Ident::Named(_), Ident::Named(_)) => false,
            _ => self == other,
        }
    }

    fn text(&self) -> String {
        match self {
            Ident::Brick(n) | Ident::Named(n) => format!("brick {n}"),
            Ident::Rule(_, k) => {
                let kinds = match &k.kinds {
                    Some(k) if !k.is_empty() => format!(" [{}]", k.join(",")),
                    _ => String::new(),
                };
                let scope = k
                    .scope
                    .as_deref()
                    .map_or(String::new(), |s| format!(" ({s})"));
                format!("rule {} -> {}{kinds}{scope}", k.from, k.to)
            }
            Ident::Limit(_, b) => format!("limits of {b}"),
            Ident::File(f) => format!("settings of {f}"),
        }
    }
}

fn same_set(a: &Option<Vec<String>>, b: &Option<Vec<String>>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => {
            let (mut x, mut y) = (x.clone(), y.clone());
            x.sort();
            y.sort();
            x == y
        }
        _ => true,
    }
}

fn spec_key(r: &RuleSpec) -> RuleKey {
    RuleKey {
        from: r.from.clone(),
        to: r.to.clone(),
        kinds: (!r.kinds.is_empty()).then(|| r.kinds.clone()),
        scope: r.scope.clone(),
    }
}

fn limit(file: &str, bricks: &Option<String>) -> Ident {
    Ident::Limit(file.into(), bricks.clone().unwrap_or_else(|| "*".into()))
}

/// A rule also depends on the bricks it names: renaming or removing one stales the write.
fn rule_idents(file: &str, key: RuleKey) -> Vec<Ident> {
    vec![
        Ident::Named(key.from.clone()),
        Ident::Named(key.to.clone()),
        Ident::Rule(file.into(), key),
    ]
}

fn touched(op: &Op) -> Vec<Ident> {
    let brick = |n: &String| Ident::Brick(n.clone());
    match op {
        Op::AddBrick { brick: b, .. } => vec![Ident::Brick(b.name.clone())],
        Op::RemoveBrick { name, .. }
        | Op::SetBrick { name, .. }
        | Op::MakeComposite { name, .. }
        | Op::Ungroup { name, .. } => vec![brick(name)],
        Op::RenameBrick { from, to, .. } => vec![brick(from), brick(to)],
        Op::MoveBrick { name, into, .. } => std::iter::once(brick(name))
            .chain(into.iter().map(brick))
            .collect(),
        Op::AddRule { file, rule } => rule_idents(file, spec_key(rule)),
        Op::RemoveRule { file, key } | Op::SetRule { file, key, .. } => {
            rule_idents(file, key.clone())
        }
        Op::AddComplexity { file, rule } => vec![limit(file, &rule.bricks)],
        Op::RemoveComplexity { file, bricks } | Op::SetComplexity { file, bricks, .. } => {
            vec![limit(file, bricks)]
        }
        Op::SetExports { file, .. }
        | Op::SetDeps { file, .. }
        | Op::AddInclude { file, .. }
        | Op::RemoveInclude { file, .. }
        | Op::CreateFile { file } => vec![Ident::File(file.clone())],
    }
}

/// `<author> v<N> changed <identity>` for the first version after `base` that touched what
/// `ops` touch; `None` when the write is safe to re-send.
pub fn first_conflict(log: &[Version], base: u64, ops: &[Op]) -> Option<String> {
    let mine: Vec<Ident> = ops.iter().flat_map(touched).collect();
    for v in log.iter().filter(|v| v.v > base) {
        let who = format!("{} v{}", v.by.name(), v.v);
        if v.ops.is_empty() {
            return Some(format!("{who} changed the design"));
        }
        let theirs = v.ops.iter().flat_map(touched);
        for t in theirs {
            if mine.iter().any(|m| m.clashes(&t)) {
                return Some(format!("{who} changed {}", t.text()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_change::{BrickSpec, By};

    fn ver(v: u64, by: By, ops: Vec<Op>) -> Version {
        Version {
            v,
            hash: String::new(),
            prev: None,
            by,
            at: 0,
            note: None,
            ops,
            diff: Vec::new(),
            files: Vec::new(),
            removed: Vec::new(),
            full: false,
        }
    }

    fn add(name: &str) -> Op {
        Op::AddBrick {
            file: "brickie.toml".into(),
            brick: BrickSpec {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    fn rule(to: &str, scope: Option<&str>) -> Op {
        Op::RemoveRule {
            file: "brickie.toml".into(),
            key: RuleKey {
                from: "web".into(),
                to: to.into(),
                kinds: None,
                scope: scope.map(Into::into),
            },
        }
    }

    #[test]
    fn disjoint_versions_allow_a_resend() {
        let log = vec![ver(2, By::Agent, vec![add("cache")])];
        assert_eq!(first_conflict(&log, 1, &[add("other")]), None);
        assert_eq!(
            first_conflict(&log, 2, &[add("cache")]),
            None,
            "before base"
        );
    }

    #[test]
    fn the_same_brick_conflicts_and_names_author_and_identity() {
        let log = vec![
            ver(2, By::Human, vec![add("x")]),
            ver(3, By::Agent, vec![add("cache")]),
        ];
        assert_eq!(
            first_conflict(&log, 1, &[add("cache")]).as_deref(),
            Some("agent v3 changed brick cache")
        );
    }

    #[test]
    fn rules_match_by_file_and_key() {
        let log = vec![ver(2, By::Agent, vec![rule("store", Some("prod"))])];
        assert!(first_conflict(&log, 1, &[rule("store", None)]).is_some());
        assert_eq!(first_conflict(&log, 1, &[rule("store", Some("dev"))]), None);
        assert_eq!(first_conflict(&log, 1, &[rule("db", None)]), None);
        assert_eq!(
            first_conflict(&log, 1, &[rule("store", None)]).as_deref(),
            Some("agent v2 changed rule web -> store (prod)")
        );
    }

    #[test]
    fn a_rule_conflicts_with_a_rename_or_removal_of_a_brick_it_names() {
        let rm = Op::RemoveBrick {
            file: "brickie.toml".into(),
            name: "store".into(),
        };
        let log = vec![ver(2, By::Human, vec![rm])];
        let add_rule = Op::AddRule {
            file: "brickie.toml".into(),
            rule: RuleSpec {
                from: "web".into(),
                to: "store".into(),
                ..Default::default()
            },
        };
        assert_eq!(
            first_conflict(&log, 1, &[add_rule]).as_deref(),
            Some("human v2 changed brick store")
        );
    }

    #[test]
    fn limits_match_by_bricks_and_opless_versions_conflict() {
        let lim = |b: Option<&str>| Op::RemoveComplexity {
            file: "brickie.toml".into(),
            bricks: b.map(Into::into),
        };
        let log = vec![ver(2, By::Agent, vec![lim(Some("web"))])];
        assert!(first_conflict(&log, 1, &[lim(Some("web"))]).is_some());
        assert_eq!(first_conflict(&log, 1, &[lim(None)]), None);
        let hand = vec![ver(2, By::External, Vec::new())];
        assert_eq!(
            first_conflict(&hand, 1, &[add("a")]).as_deref(),
            Some("external v2 changed the design")
        );
    }
}
