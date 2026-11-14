//! Planned and unused links grouped per (from, to) brick pair: one line per pair, the worst
//! state wins (unused over planned), and the tooltip lists every rule.

use crate::state::*;
use brickie_web_api::PlannedEdge;

/// The longest reason a label shows (characters); the tooltip has the full text.
pub const LABEL_MAX: usize = 40;

/// The bricks of the level a rule selector names (`*` names none, ghosts never).
pub fn ends(cx: &Cx, sel: &str) -> Vec<usize> {
    (0..cx.m.g.bricks.len())
        .filter(|&i| !cx.m.g.bricks[i].ghost && sel != "*" && names(sel, &cx.m.g.bricks[i]))
        .collect()
}

/// The rules that draw between bricks `a` and `b`, in rule order.
pub struct Pair<'a> {
    pub a: usize,
    pub b: usize,
    pub rules: Vec<&'a PlannedEdge>,
}

impl Pair<'_> {
    /// `unused` when any rule is unused, else `planned`.
    pub fn state(&self) -> &'static str {
        worst(self.rules.iter().map(|e| e.state.as_str()))
    }

    /// The rule that decides the line: the first one in the worst state.
    pub fn lead(&self) -> &PlannedEdge {
        let w = self.state();
        self.rules
            .iter()
            .find(|e| e.state == w)
            .unwrap_or(&self.rules[0])
    }

    pub fn no_reason(&self) -> bool {
        self.rules.iter().any(|e| e.reason.is_none())
    }

    /// The reason the label shows: the first rule's that has one, cut, with the count of the
    /// other rules.
    pub fn label(&self) -> Option<String> {
        let reason = self.rules.iter().find_map(|e| e.reason.as_deref())?;
        let more = self.rules.len() - 1;
        let tail = if more > 0 {
            format!(" (+{more})")
        } else {
            String::new()
        };
        Some(format!("{}{tail}", cut(reason, LABEL_MAX)))
    }

    /// Every rule with its state, reason and filter text, one per line.
    pub fn tip(&self) -> String {
        let line = |e: &PlannedEdge| {
            let why = e.reason.as_deref().unwrap_or("no reason yet");
            format!(
                "{} \u{2192} {} [{}] {why} ({})",
                e.from, e.to, e.state, e.filter
            )
        };
        self.rules
            .iter()
            .map(|e| line(e))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The worst of link states: `unused` beats `planned`.
pub fn worst<'a>(states: impl IntoIterator<Item = &'a str>) -> &'static str {
    if states.into_iter().any(|s| s == "unused") {
        "unused"
    } else {
        "planned"
    }
}

/// `s` cut to `max` characters with an ellipsis.
pub fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{}\u{2026}", head.trim_end())
}

/// The links of the current level grouped per pair, in rule order.
pub fn pairs(cx: &Cx) -> Vec<Pair<'_>> {
    let mut out: Vec<Pair> = Vec::new();
    for e in &cx.level().planned_edges {
        for a in ends(cx, &e.from) {
            for b in ends(cx, &e.to).into_iter().filter(|&b| b != a) {
                match out.iter_mut().find(|p| (p.a, p.b) == (a, b)) {
                    Some(p) => p.rules.push(e),
                    None => out.push(Pair {
                        a,
                        b,
                        rules: vec![e],
                    }),
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(state: &str, reason: Option<&str>) -> PlannedEdge {
        PlannedEdge {
            level: "root".into(),
            file: "brickie.toml".into(),
            from: "a".into(),
            to: "b".into(),
            kinds: vec![],
            scope: "prod".into(),
            action: "allow".into(),
            reason: reason.map(String::from),
            state: state.into(),
            filter: "no edge in prod scope".into(),
        }
    }

    #[test]
    fn the_worst_state_wins_per_pair() {
        assert_eq!(worst(["planned", "planned"]), "planned");
        assert_eq!(worst(["planned", "unused"]), "unused");
        let (p, u) = (edge("planned", Some("x")), edge("unused", None));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &u],
        };
        assert_eq!(pair.state(), "unused");
        assert!(std::ptr::eq(pair.lead(), &u));
        assert!(pair.no_reason());
        let tip = pair.tip();
        assert!(tip.contains("[planned] x") && tip.contains("[unused] no reason yet"));
    }

    #[test]
    fn long_reasons_are_cut_and_short_ones_kept() {
        assert_eq!(cut("short", LABEL_MAX), "short");
        let long = "x".repeat(60);
        let c = cut(&long, LABEL_MAX);
        assert_eq!(c.chars().count(), LABEL_MAX);
        assert!(c.ends_with('\u{2026}'));
        let (p, q) = (edge("planned", Some("why")), edge("planned", None));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &q],
        };
        assert_eq!(pair.label().as_deref(), Some("why (+1)"));
    }
}
