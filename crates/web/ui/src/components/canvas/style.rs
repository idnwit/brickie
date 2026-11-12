//! The pure decisions behind a brick's and an edge's DOM: class strings, marker ids, the score
//! badge, the heat bar, the name fit. Plain values in, plain values out (no signals, no `Cx`);
//! `brick.rs` and `edge.rs` read the reactive state into these and render the result.

use crate::state::EStat;
use brickie_web_api::{fmt_score, heat_len, heat_level};

/// What decides a brick's state classes, read from the evaluation.
#[derive(Default, Clone, Copy)]
pub struct NodeFlags {
    /// The focus lights other bricks, not this one.
    pub off_focus: bool,
    /// The Violations preset is active.
    pub viol: bool,
    /// Open findings on the brick, and whether one of them is an error.
    pub open: usize,
    pub err: bool,
    /// Selected (or part of the focused finding while an item is open).
    pub selected: bool,
    /// Accepted debt only.
    pub acc: bool,
    /// The hovered finding touches the brick.
    pub hov: bool,
}

impl NodeFlags {
    /// `node{kind}` plus ` dh`, ` sel`, ` bd-red`/` bd-amber`/` acc`, ` hov`, in that order.
    pub fn class(&self, kind: &str) -> String {
        let mut c = format!("node{kind}");
        if self.off_focus || (self.viol && self.open == 0) {
            c += " dh";
        }
        if self.selected {
            c += " sel";
        }
        if self.open > 0 {
            c += if self.err { " bd-red" } else { " bd-amber" };
        } else if self.acc {
            c += " acc";
        }
        if self.hov {
            c += " hov";
        }
        c
    }
}

/// The corner badge: label, class, and the box and text x for a brick `w` wide.
#[derive(Debug, PartialEq)]
pub struct Badge {
    pub label: String,
    pub cls: &'static str,
    pub bw: f64,
    pub x: f64,
    pub tx: f64,
}

impl Badge {
    /// `3E 1W` of the open errors and warnings, a check when clean.
    pub fn new(e: u32, w: u32, box_w: f64) -> Self {
        let (label, cls) = match (e, w) {
            (0, 0) => ("\u{2713}".to_string(), "sbadge clean"),
            (_, 0) => (format!("{e}E"), "sbadge err"),
            (0, _) => (format!("{w}W"), "sbadge warn"),
            _ => (format!("{e}E {w}W"), "sbadge err"),
        };
        let bw = (10 + 7 * label.chars().count()).max(22) as f64;
        Self {
            label,
            cls,
            bw,
            x: box_w - bw - 6.0,
            tx: box_w - 6.0 - bw / 2.0,
        }
    }
}

/// The heat bar of a brick `w` wide: class, tooltip, track and fill length.
#[derive(Debug, PartialEq)]
pub struct Heat {
    pub cls: String,
    pub tip: String,
    pub track: f64,
    pub fill: f64,
}

impl Heat {
    /// Length and intensity follow the worst value/limit ratio `worst`; any ratio above 0 shows
    /// at least 2 px.
    pub fn new(worst: f64, w: f64) -> Self {
        let track = w - 16.0;
        let tip = if worst > 0.0 {
            format!("worst function is at {}x its limit", fmt_score(worst))
        } else {
            "no complexity limit applies".to_string()
        };
        Self {
            cls: format!("heat h{}", heat_level(worst)),
            tip,
            track,
            fill: (track * heat_len(worst)).max(if worst > 0.0 { 2.0 } else { 0.0 }),
        }
    }
}

/// The name shown in a box `w` wide: without the `root/` prefix, cut with an ellipsis past the
/// fit (at least 8 chars).
pub fn fit_name(name: &str, w: f64) -> String {
    let fit = (((w - 54.0) / 8.2) as usize).max(8);
    let shown = name.strip_prefix("root/").unwrap_or(name);
    if shown.chars().count() > fit {
        format!(
            "{}\u{2026}",
            shown.chars().take(fit - 1).collect::<String>()
        )
    } else {
        shown.to_string()
    }
}

/// What decides an edge's state classes and markers.
#[derive(Default, Clone, Copy, PartialEq)]
pub struct EdgeFlags {
    /// The status it is drawn with, `None` when hidden.
    pub st: Option<EStat>,
    /// Private: stops at the wall of the level.
    pub wall: bool,
    /// An open error (or no finding at all) backs a violating edge.
    pub err: bool,
    /// The Architecture preset: kind colours.
    pub arch: bool,
    /// Hot in the focus, and whether the focus is a single edge (shows the label).
    pub hot: bool,
    pub lab: bool,
    pub sel: bool,
    pub hov: bool,
}

impl EdgeFlags {
    /// `edge k-{kind}` with ` wall`, the status, ` kc`, ` hot`, ` lab`, ` sel`, ` hov`, in that
    /// order; `edge off` when hidden.
    pub fn class(&self, kind: &str) -> String {
        let Some(st) = self.st else {
            return "edge off".to_string();
        };
        let mut c = format!("edge k-{kind}");
        if self.wall {
            c += " wall";
        }
        c += match st {
            EStat::Ok => " ok",
            EStat::Accepted => " acc",
            EStat::Debt if self.err => " err",
            EStat::Debt => " warn",
        };
        for (on, s) in [
            (self.arch, " kc"),
            (self.hot, " hot"),
            (self.lab, " lab"),
            (self.sel, " sel"),
            (self.hov, " hov"),
        ] {
            if on {
                c += s;
            }
        }
        c
    }

    /// The arrowhead: violating and accepted edges get their own colour, allowed ones the kind
    /// colour in Architecture and grey elsewhere; none on a wall edge.
    pub fn marker_end(&self, kind: &str) -> String {
        if self.wall {
            return String::new();
        }
        let id = match self.st {
            Some(EStat::Accepted) => "acc".to_string(),
            Some(EStat::Debt) if self.err => "viol".to_string(),
            Some(EStat::Debt) => "warn".to_string(),
            _ if self.arch => kind.to_string(),
            _ => format!("n-{kind}"),
        };
        format!("url(#m-{id})")
    }

    /// The tail mark of a re-export, none for other kinds.
    pub fn marker_start(&self, kind: &str) -> &'static str {
        match (kind == "reexports", self.arch) {
            (false, _) => "",
            (true, true) => "url(#t-reexports)",
            (true, false) => "url(#t-n-reexports)",
        }
    }

    /// The mark at the label: a cross on debt, a check when accepted.
    pub fn mark(&self) -> &'static str {
        match self.st {
            Some(EStat::Debt) => "\u{2717}",
            Some(EStat::Accepted) => "\u{2713}",
            _ => "",
        }
    }
}

/// The stroke-width style: grows with the number of references (log scale, at most 2.6x).
pub fn edge_width(count: usize) -> String {
    let k = (1.0 + 0.35 * (count.max(1) as f64).log2()).min(2.6);
    format!("--k:{k:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_class_order() {
        let f = NodeFlags {
            off_focus: true,
            selected: true,
            open: 2,
            err: true,
            hov: true,
            ..Default::default()
        };
        assert_eq!(f.class(" comp"), "node comp dh sel bd-red hov");
        let f = NodeFlags {
            open: 1,
            ..Default::default()
        };
        assert_eq!(f.class(""), "node bd-amber");
        let f = NodeFlags {
            viol: true,
            acc: true,
            ..Default::default()
        };
        assert_eq!(f.class(""), "node dh acc");
        assert_eq!(NodeFlags::default().class(""), "node");
    }

    #[test]
    fn badge_label_and_place() {
        let b = Badge::new(0, 0, 100.0);
        assert_eq!(
            (b.label.as_str(), b.cls, b.bw),
            ("\u{2713}", "sbadge clean", 22.0)
        );
        assert_eq!((b.x, b.tx), (72.0, 83.0));
        let b = Badge::new(3, 1, 200.0);
        assert_eq!(
            (b.label.as_str(), b.cls, b.bw),
            ("3E 1W", "sbadge err", 45.0)
        );
        assert_eq!(Badge::new(0, 2, 100.0).label, "2W");
        assert_eq!(Badge::new(0, 2, 100.0).cls, "sbadge warn");
        assert_eq!(Badge::new(4, 0, 100.0).cls, "sbadge err");
    }

    #[test]
    fn heat_bar() {
        let h = Heat::new(0.0, 116.0);
        assert_eq!((h.track, h.fill), (100.0, 0.0));
        assert_eq!(h.tip, "no complexity limit applies");
        let h = Heat::new(0.0001, 116.0);
        assert!(h.fill >= 2.0);
        assert!(h.tip.starts_with("worst function is at "));
    }

    #[test]
    fn name_fit() {
        assert_eq!(fit_name("root/app", 200.0), "app");
        // width 100 fits (100-54)/8.2 = 5 -> raised to 8 chars: 7 kept plus an ellipsis.
        assert_eq!(fit_name("abcdefghij", 100.0), "abcdefg\u{2026}");
        assert_eq!(fit_name("abcdefgh", 100.0), "abcdefgh");
    }

    #[test]
    fn edge_class_order() {
        let f = EdgeFlags {
            st: Some(EStat::Debt),
            wall: true,
            err: true,
            arch: true,
            hot: true,
            lab: true,
            sel: true,
            hov: true,
        };
        assert_eq!(f.class("uses"), "edge k-uses wall err kc hot lab sel hov");
        let f = EdgeFlags {
            st: Some(EStat::Debt),
            ..Default::default()
        };
        assert_eq!(f.class("uses"), "edge k-uses warn");
        let f = EdgeFlags {
            st: Some(EStat::Accepted),
            ..Default::default()
        };
        assert_eq!(f.class("uses"), "edge k-uses acc");
        assert_eq!(EdgeFlags::default().class("uses"), "edge off");
    }

    #[test]
    fn edge_markers() {
        let mut f = EdgeFlags {
            st: Some(EStat::Ok),
            ..Default::default()
        };
        assert_eq!(f.marker_end("uses"), "url(#m-n-uses)");
        f.arch = true;
        assert_eq!(f.marker_end("uses"), "url(#m-uses)");
        assert_eq!(f.marker_start("reexports"), "url(#t-reexports)");
        assert_eq!(f.marker_start("uses"), "");
        f.arch = false;
        assert_eq!(f.marker_start("reexports"), "url(#t-n-reexports)");
        f.st = Some(EStat::Debt);
        assert_eq!(f.marker_end("uses"), "url(#m-warn)");
        f.err = true;
        assert_eq!(f.marker_end("uses"), "url(#m-viol)");
        f.st = Some(EStat::Accepted);
        assert_eq!(f.marker_end("uses"), "url(#m-acc)");
        f.wall = true;
        assert_eq!(f.marker_end("uses"), "");
    }

    #[test]
    fn edge_mark_and_width() {
        let f = |st| EdgeFlags {
            st,
            ..Default::default()
        };
        assert_eq!(f(Some(EStat::Debt)).mark(), "\u{2717}");
        assert_eq!(f(Some(EStat::Accepted)).mark(), "\u{2713}");
        assert_eq!(f(Some(EStat::Ok)).mark(), "");
        assert_eq!(edge_width(0), "--k:1.00");
        assert_eq!(edge_width(2), "--k:1.35");
        assert_eq!(edge_width(1 << 20), "--k:2.60");
    }
}
