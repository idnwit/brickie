//! Links grouped per (from, to) brick pair: one line per pair, and the tooltip lists every
//! link.

use crate::state::*;
use brickie_web_api::LevelLink;

/// The longest reason a label shows (characters); the tooltip has the full text.
pub const LABEL_MAX: usize = 40;

/// The brick of the level a link end names (`composite/child` lands on the composite; ghosts
/// never).
pub fn end(cx: &Cx, name: &str) -> Option<usize> {
    let name = link_brick(name);
    (0..cx.m.g.bricks.len()).find(|&i| !cx.m.g.bricks[i].ghost && cx.m.g.bricks[i].name == name)
}

/// The links that draw between bricks `a` and `b`, in file order.
pub struct Pair<'a> {
    pub a: usize,
    pub b: usize,
    pub rules: Vec<&'a LevelLink>,
}

impl Pair<'_> {
    /// The link that decides the line: the first one.
    pub fn lead(&self) -> &LevelLink {
        self.rules[0]
    }

    /// Some link of the pair no rule allows (`design/link-denied`).
    pub fn denied(&self) -> bool {
        self.rules.iter().any(|e| e.denied.is_some())
    }

    /// Some link of the pair sits on a cycle of the level's links.
    pub fn cycle(&self) -> bool {
        self.rules.iter().any(|e| e.cycle)
    }

    /// The first link's reason, cut, with the count of the other links.
    pub fn label(&self) -> Option<String> {
        let reason = self.lead().reason.as_str();
        let more = self.rules.len() - 1;
        let tail = if more > 0 {
            format!(" (+{more})")
        } else {
            String::new()
        };
        Some(format!("{}{tail}", cut(reason, LABEL_MAX)))
    }

    /// Every link with its reason, filter text and why it is denied, one per line.
    pub fn tip(&self) -> String {
        let line = |e: &LevelLink| {
            let mut t = format!(
                "link {} \u{2192} {} {} ({})",
                e.from, e.to, e.reason, e.filter
            );
            if let Some(why) = &e.denied {
                t.push_str(&format!("\n  not allowed: {why}"));
            }
            if e.cycle {
                t.push_str("\n  on a cycle of this level's links");
            }
            t
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

/// The drawn links of the current level grouped per pair, in file order.
pub fn pairs(cx: &Cx) -> Vec<Pair<'_>> {
    let mut out: Vec<Pair> = Vec::new();
    let editing = cx.editing();
    for e in cx.level().links.iter().filter(|e| drawn(e, editing)) {
        let (Some(a), Some(b)) = (end(cx, &e.from), end(cx, &e.to)) else {
            continue;
        };
        if a == b {
            continue;
        }
        match out.iter_mut().find(|p| (p.a, p.b) == (a, b)) {
            Some(p) => p.rules.push(e),
            None => out.push(Pair {
                a,
                b,
                rules: vec![e],
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(reason: &str, denied: Option<&str>) -> LevelLink {
        LevelLink {
            level: "root".into(),
            file: "brickie.toml".into(),
            from: "a".into(),
            to: "b".into(),
            kinds: vec![],
            scope: "all".into(),
            reason: reason.into(),
            filter: "edge in any scope".into(),
            denied: denied.map(String::from),
            cycle: false,
            planned: false,
        }
    }

    #[test]
    fn the_tip_lists_every_link_and_why_it_is_denied() {
        let (p, u) = (edge("x", None), edge("y", Some("no rule allows a -> b")));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &u],
        };
        assert!(pair.denied() && !pair.cycle());
        let tip = pair.tip();
        assert!(tip.contains("a \u{2192} b x (") && tip.contains("not allowed: no rule allows"));
    }

    #[test]
    fn long_reasons_are_cut_and_short_ones_kept() {
        assert_eq!(cut("short", LABEL_MAX), "short");
        let long = "x".repeat(60);
        let c = cut(&long, LABEL_MAX);
        assert_eq!(c.chars().count(), LABEL_MAX);
        assert!(c.ends_with('\u{2026}'));
        let (p, q) = (edge("why", None), edge("other", None));
        let pair = Pair {
            a: 0,
            b: 1,
            rules: vec![&p, &q],
        };
        assert_eq!(pair.label().as_deref(), Some("why (+1)"));
    }
}
