//! Ops in plain words (the Ops tab and the review bar), the brick an op is about, and the
//! agent proposals waiting for review.

use brickie_web_api::ChangeView;
use brickie_web_api::change::{By, Op, OpRecord, RuleSpec, State};

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
    format!("{verb} {} \u{2192} {} ({kinds}, {scope})", r.from, r.to)
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

/// The op in plain words: "Added brick telemetry", "Allowed a -> b (uses, prod)".
pub fn describe(op: &Op) -> String {
    if let Some(w) = brick_words(op) {
        return w;
    }
    match op {
        Op::AddRule { rule, .. } => rule_words(rule),
        Op::SetRule { rule, index, .. } => format!("Changed rule #{index}: {}", rule_words(rule)),
        Op::RemoveRule { index, .. } => format!("Removed rule #{index}"),
        Op::SetExports { exports, .. } => format!("Set exports ({} bricks)", exports.len()),
        Op::AddComplexity { .. } => "Added a complexity rule".into(),
        Op::SetComplexity { index, .. } => format!("Changed complexity rule #{index}"),
        Op::RemoveComplexity { index, .. } => format!("Removed complexity rule #{index}"),
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

/// An agent op still waiting for review.
pub fn pending(r: &OpRecord) -> bool {
    r.by == By::Agent && r.state == State::Proposed
}

/// The level whose own file the op edits.
pub fn op_level(ch: &ChangeView, r: &OpRecord) -> Option<String> {
    ch.files
        .iter()
        .find(|(_, f)| **f == r.file)
        .map(|(l, _)| l.clone())
}

/// Pending agent ops about brick `name` of level `lv`.
pub fn proposals_on<'a>(ch: &'a ChangeView, lv: &str, name: &str) -> Vec<&'a OpRecord> {
    let file = ch.files.get(lv);
    ch.log
        .iter()
        .filter(|r| pending(r) && Some(&r.file) == file && op_brick(&r.op) == Some(name))
        .collect()
}

/// The first pending agent op after `seq` (wrapping around).
pub fn next_proposal(ch: &ChangeView, seq: u64) -> Option<&OpRecord> {
    let open: Vec<&OpRecord> = ch.log.iter().filter(|r| pending(r)).collect();
    open.iter().find(|r| r.seq > seq).or(open.first()).copied()
}

/// Whether the Ops tab filter keeps record `r`.
pub fn keep_op(filter: &str, r: &OpRecord) -> bool {
    match filter {
        "agent" => r.by == By::Agent,
        "human" => r.by == By::Human,
        "proposed" => r.state == State::Proposed,
        "conflict" => r.conflict.is_some(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_web_api::change::BrickSpec;

    fn rec(seq: u64, by: By, state: State, op: Op) -> OpRecord {
        OpRecord {
            seq,
            by,
            at: 0,
            file: op.file().to_string(),
            op,
            key: String::new(),
            before: None,
            after: None,
            note: None,
            state,
            review: None,
            conflict: None,
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
        assert_eq!(op_brick(&add("t")), Some("t"));
        assert_eq!(op_brick(&r), None);
    }

    #[test]
    fn proposals_cycle_and_filter() {
        let log = [
            rec(1, By::Agent, State::Proposed, add("a")),
            rec(2, By::Human, State::Accepted, add("b")),
            rec(3, By::Agent, State::Proposed, add("c")),
        ];
        let ch = ChangeView {
            log: log.to_vec(),
            ..super::super::sheet::tests::change()
        };
        assert_eq!(next_proposal(&ch, 1).map(|r| r.seq), Some(3));
        assert_eq!(next_proposal(&ch, 3).map(|r| r.seq), Some(1));
        assert_eq!(proposals_on(&ch, "root", "c").len(), 1);
        assert!(keep_op("human", &log[1]) && !keep_op("agent", &log[1]));
    }
}
