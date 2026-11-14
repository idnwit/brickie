//! Ops and versions in plain words (toasts, the History tab, undo tooltips) and the brick an
//! op is about.

use brickie_web_api::design::{By, Op, RuleKey, RuleSpec, Version};

fn rule_words(r: &RuleSpec) -> String {
    let kinds = if r.kinds.is_empty() {
        "all kinds".to_string()
    } else {
        r.kinds.join("+")
    };
    let verb = if r.action == "deny" {
        "Denied"
    } else {
        "Allowed"
    };
    let scope = r.scope.as_deref().unwrap_or("prod");
    let via = if r.via.is_empty() {
        String::new()
    } else {
        format!(" via {}", r.via.join("+"))
    };
    format!(
        "{verb} {} \u{2192} {}{via} ({kinds}, {scope})",
        r.from, r.to
    )
}

fn brick_words(op: &Op) -> Option<String> {
    Some(match op {
        Op::AddBrick { brick, .. } => format!("Added brick {}", brick.name),
        Op::RemoveBrick { name, .. } => format!("Removed brick {name}"),
        Op::SetBrick { name, .. } => format!("Edited brick {name}"),
        Op::RenameBrick { from, to, .. } => format!("Renamed {from} to {to}"),
        Op::MoveBrick {
            name,
            into,
            into_file,
            ..
        } => {
            format!("Moved {name} into {}", into.as_deref().unwrap_or(into_file))
        }
        Op::MakeComposite { name, .. } => format!("Made {name} a group"),
        Op::Ungroup { name, .. } => format!("Ungrouped {name}"),
        _ => return None,
    })
}

/// `a \u{2192} b`.
fn key_words(k: &RuleKey) -> String {
    format!("{} \u{2192} {}", k.from, k.to)
}

/// The op in plain words: "Added brick telemetry", "Allowed a -> b (uses, prod)".
pub fn describe(op: &Op) -> String {
    if let Some(w) = brick_words(op) {
        return w;
    }
    match op {
        Op::AddRule { rule, .. } => rule_words(rule),
        Op::SetRule { key, .. } => format!("Changed rule {}", key_words(key)),
        Op::RemoveRule { key, .. } => format!("Removed rule {}", key_words(key)),
        Op::SetExports { exports, .. } => format!("Set exports ({} bricks)", exports.len()),
        Op::AddComplexity { .. } => "Added a complexity rule".into(),
        Op::SetComplexity { bricks, .. } => {
            format!("Changed complexity of {}", bricks.as_deref().unwrap_or("*"))
        }
        Op::RemoveComplexity { bricks, .. } => {
            format!("Removed complexity of {}", bricks.as_deref().unwrap_or("*"))
        }
        Op::SetDeps { .. } => "Changed deps defaults".into(),
        Op::AddInclude { path, .. } => format!("Included {path}"),
        Op::RemoveInclude { path, .. } => format!("Dropped include {path}"),
        Op::CreateFile { file } => format!("Created {file}"),
        _ => op.name().to_string(),
    }
}

/// The brick an op is about (by name in its file), if any.
pub fn op_brick(op: &Op) -> Option<&str> {
    match op {
        Op::AddBrick { brick, .. } => Some(&brick.name),
        Op::RemoveBrick { name, .. }
        | Op::SetBrick { name, .. }
        | Op::MoveBrick { name, .. }
        | Op::MakeComposite { name, .. }
        | Op::Ungroup { name, .. } => Some(name),
        Op::RenameBrick { to, .. } => Some(to),
        _ => None,
    }
}

/// The ops of the agent's versions since the human's last version (what the canvas marks as
/// the agent's).
pub fn agent_ops(history: &[Version]) -> Vec<&Op> {
    let after = history
        .iter()
        .rposition(|v| v.by == By::Human)
        .map_or(0, |i| i + 1);
    (history[after..].iter())
        .filter(|v| v.by == By::Agent)
        .flat_map(|v| &v.ops)
        .collect()
}

/// Who made a version, as the History tab and toasts name it.
pub fn by_word(by: By) -> &'static str {
    match by {
        By::Human => "you",
        By::Agent => "agent",
        By::External => "hand edit",
    }
}

/// What a version did, in one line: its first op, else its note, else its first diff line
/// (`+N more` when there is more).
pub fn version_words(v: &Version) -> String {
    let (first, n) = match (v.ops.first(), v.note.as_deref(), v.diff.first()) {
        (Some(op), _, _) => (describe(op), v.ops.len()),
        (None, Some(note), _) => (note.to_string(), 1),
        (None, None, Some(d)) => (d.clone(), v.diff.len()),
        _ => ("no change".into(), 1),
    };
    match n {
        1 => first,
        n => format!("{first} (+{} more)", n - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_web_api::design::BrickSpec;

    fn add(name: &str) -> Op {
        Op::AddBrick {
            file: "brickie.toml".into(),
            brick: BrickSpec {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn ops_read_as_words() {
        assert_eq!(describe(&add("telemetry")), "Added brick telemetry");
        let r = Op::AddRule {
            file: "x".into(),
            rule: RuleSpec {
                from: "a".into(),
                to: "b".into(),
                kinds: vec!["uses".into()],
                scope: Some("prod".into()),
                action: "allow".into(),
                ..Default::default()
            },
        };
        assert_eq!(describe(&r), "Allowed a \u{2192} b (uses, prod)");
        let v = Op::AddRule {
            file: "x".into(),
            rule: RuleSpec {
                from: "ui".into(),
                to: "application".into(),
                via: vec!["ports-in".into()],
                action: "allow".into(),
                ..Default::default()
            },
        };
        assert_eq!(
            describe(&v),
            "Allowed ui \u{2192} application via ports-in (all kinds, prod)"
        );
        assert_eq!(op_brick(&add("t")), Some("t"));
        assert_eq!(op_brick(&r), None);
    }

    #[test]
    fn versions_read_as_one_line() {
        let mut v = Version {
            v: 3,
            hash: "ab".into(),
            prev: None,
            by: By::Agent,
            at: 0,
            note: None,
            ops: vec![add("a"), add("b")],
            diff: vec![],
            files: vec![],
            removed: vec![],
            full: false,
        };
        assert_eq!(version_words(&v), "Added brick a (+1 more)");
        v.ops.clear();
        v.note = Some("undo v2".into());
        assert_eq!(version_words(&v), "undo v2");
        assert_eq!(by_word(By::External), "hand edit");
        let mut human = v.clone();
        human.by = By::Human;
        v.ops = vec![add("c")];
        assert_eq!(agent_ops(&[human.clone(), v.clone()]).len(), 1);
        assert!(agent_ops(&[v, human]).is_empty());
    }
}
