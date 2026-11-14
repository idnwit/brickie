//! Undo / redo as versions: undo reverts the human's latest edit (a new version `undo vN`),
//! redo reverts that undo. Both stacks are read from the history, so they survive a reload and
//! skip what an agent or a hand edit did in between.

use super::version_words;
use brickie_web_api::design::{By, Version};

/// `undo v12` -> 12.
fn undid(v: &Version) -> Option<u64> {
    v.note.as_deref()?.strip_prefix("undo v")?.parse().ok()
}

/// `(undo, redo)` stacks of the human's versions (tops last): an edit is undone by reverting
/// it; an undo is redone by reverting the undo.
pub fn stacks(history: &[Version]) -> (Vec<u64>, Vec<u64>) {
    let (mut undo, mut redo) = (Vec::new(), Vec::new());
    for v in history.iter().filter(|v| v.by == By::Human) {
        match undid(v) {
            Some(n) if redo.contains(&n) => {
                redo.retain(|x| *x != n);
                undo.push(v.v);
            }
            Some(n) => {
                undo.retain(|x| *x != n);
                redo.push(v.v);
            }
            None if !v.ops.is_empty() => {
                undo.push(v.v);
                redo.clear();
            }
            None => {}
        }
    }
    (undo, redo)
}

/// The version undo would revert.
pub fn undo_target(history: &[Version]) -> Option<u64> {
    stacks(history).0.last().copied()
}

/// The version redo would revert (an undo).
pub fn redo_target(history: &[Version]) -> Option<u64> {
    stacks(history).1.last().copied()
}

/// What reverting `v` takes back, in words: an edit's own words, or for an undo (or a redo)
/// the words of the edit it reverted.
fn what(history: &[Version], v: u64) -> String {
    let find = |n: u64| history.iter().find(|x| x.v == n);
    let mut cur = find(v);
    while let Some(n) = cur.and_then(undid) {
        match find(n) {
            Some(x) => cur = Some(x),
            None => break,
        }
    }
    cur.map(version_words).unwrap_or_default()
}

/// `Undo: Added brick tel` / `Redo: ...`, or why it is off.
pub fn undo_tip(word: &str, history: &[Version], target: Option<u64>) -> String {
    match target {
        Some(v) => format!("{word}: {}", what(history, v)),
        None => format!("Nothing to {}", word.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_web_api::design::Op;

    fn ver(v: u64, by: By, note: Option<&str>, edit: bool) -> Version {
        Version {
            v,
            hash: String::new(),
            prev: None,
            by,
            at: 0,
            note: note.map(String::from),
            ops: match edit {
                true => vec![Op::CreateFile {
                    file: "a.toml".into(),
                }],
                false => vec![],
            },
            diff: vec![],
            files: vec![],
            removed: vec![],
            full: false,
        }
    }

    #[test]
    fn undo_and_redo_walk_the_human_versions() {
        let mut h = vec![
            ver(1, By::External, Some("first version"), false),
            ver(2, By::Human, None, true),
            ver(3, By::Agent, None, true),
            ver(4, By::Human, None, true),
        ];
        assert_eq!((undo_target(&h), redo_target(&h)), (Some(4), None));
        h.push(ver(5, By::Human, Some("undo v4"), false));
        assert_eq!((undo_target(&h), redo_target(&h)), (Some(2), Some(5)));
        assert_eq!(undo_tip("Redo", &h, Some(5)), "Redo: Created a.toml");
        // Redo: revert the undo; undo now targets the redo.
        h.push(ver(6, By::Human, Some("undo v5"), false));
        assert_eq!((undo_target(&h), redo_target(&h)), (Some(6), None));
        // A new edit clears redo.
        h.push(ver(7, By::Human, Some("undo v6"), false));
        h.push(ver(8, By::Human, None, true));
        assert_eq!((undo_target(&h), redo_target(&h)), (Some(8), None));
        assert_eq!(undo_tip("Redo", &h, None), "Nothing to redo");
    }
}
