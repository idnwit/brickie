//! Unused links grouped per (from, to) brick pair: one line per pair, and the tooltip lists
//! every rule.

use crate::state::*;
use brickie_web_api::UnusedLink;

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
    pub rules: Vec<&'a UnusedLink>,
}

impl Pair<'_> {
    /// The rule that decides the line: the first one.
    pub fn lead(&self) -> &UnusedLink {
        self.rules[0]
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

    /// Every rule with its reason and filter text, one per line.
    pub fn tip(&self) -> String {
        let line = |e: &UnusedLink| {
            let why = e.reason.as_deref().unwrap_or("no reason yet");
            format!("{} \u{2192} {} {why} ({})", e.from, e.to, e.filter)
        };
        self.rules
            .iter()
            .map(|e| line(e))
            .collect::<Vec<_>>()
            .join("\n")
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
    for e in &cx.level().unused_links {
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

    fn edge(reason: Option<&str>) -> UnusedLink {
        UnusedLink {
            level: "root".into(),
            file: "brickie.toml".into(),
            from: "a".into(),
            to: "b".into(),
            kinds: vec![],
            scope: "prod".into(),
            action: "allow".into(),
            reason: reason.map(String::from),
            filter: "no edge in prod scope".into(),
        }
    }

    #[test]
    fn the_tip_lists_every_rule_with_its_reason_or_none() {
        let (p, u) = (edge(Some("x")), edge(None));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &u],
        };
        assert!(pair.no_reason());
        let tip = pair.tip();
        assert!(tip.contains("a \u{2192} b x (") && tip.contains("no reason yet"));
    }

    #[test]
    fn long_reasons_are_cut_and_short_ones_kept() {
        assert_eq!(cut("short", LABEL_MAX), "short");
        let long = "x".repeat(60);
        let c = cut(&long, LABEL_MAX);
        assert_eq!(c.chars().count(), LABEL_MAX);
        assert!(c.ends_with('\u{2026}'));
        let (p, q) = (edge(Some("why")), edge(None));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &q],
        };
        assert_eq!(pair.label().as_deref(), Some("why (+1)"));
    }
}
