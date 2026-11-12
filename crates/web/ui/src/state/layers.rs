//! Data layers and presets (`LDEF`, `PRESETS`, `SUBS`, `applyLay`, `layStr`, `fOn`, `vOn` of the
//! original). The URL stores only the differences from the preset (`p=viol&lay=-cx,+tests`).

use brickie_web_api::*;
use std::collections::BTreeMap;

pub const LDEF: [(&str, bool); 23] = [
    ("edges", true),
    ("tests", false),
    ("k-uses", true),
    ("k-implements", true),
    ("k-reexports", true),
    ("deps", true),
    ("d-deny", true),
    ("d-default", true),
    ("d-private", true),
    ("d-cycle", true),
    ("cx", true),
    ("c-cognitive", true),
    ("c-cyclomatic", true),
    ("c-nesting", true),
    ("c-paths", true),
    ("c-params", true),
    ("c-lines", true),
    ("stale", true),
    ("s-file", true),
    ("s-unmapped", true),
    ("s-other", true),
    ("iface", true),
    ("i-unused", true),
];

/// `PRESETS`: overrides of `LDEF`.
pub fn preset_over(p: &str) -> &'static [(&'static str, bool)] {
    match p {
        "arch" => &[("cx", false), ("stale", false), ("iface", false)],
        "viol" => &[
            ("edges", false),
            ("deps", true),
            ("cx", true),
            ("stale", true),
        ],
        _ => &[],
    }
}

pub const PRESETS: [(&str, &str); 4] = [
    ("arch", "Architecture"),
    ("viol", "Violations"),
    ("all", "All"),
    ("custom", "Custom"),
];

/// `SUBS`: the sub-layers of each finding group.
pub const SUBS: [(&str, &[(&str, &str)]); 4] = [
    (
        "deps",
        &[
            ("d-deny", "deps/deny (rule forbids)"),
            ("d-default", "deps/default (not allowed)"),
            ("d-private", "deps/private (not exported)"),
            ("d-cycle", "deps/cycle"),
        ],
    ),
    (
        "cx",
        &[
            ("c-cognitive", "cognitive"),
            ("c-cyclomatic", "cyclomatic"),
            ("c-nesting", "nesting"),
            ("c-paths", "paths"),
            ("c-params", "params"),
            ("c-lines", "lines"),
        ],
    ),
    (
        "stale",
        &[("s-file", "stale/file"), ("s-unmapped", "bricks/unmapped")],
    ),
    ("iface", &[("i-unused", "interface/unused")]),
];

pub fn group_name(g: &str) -> &'static str {
    match g {
        "deps" => "deps",
        "cx" => "complexity",
        "iface" => "interface",
        _ => "stale",
    }
}

/// The effective data layers (`S.L`).
#[derive(Debug, Clone, PartialEq)]
pub struct Lay(BTreeMap<&'static str, bool>);

impl Lay {
    /// `applyLay`: the preset, then the `+key` / `-key` differences.
    pub fn from_params(p: &Params) -> Lay {
        let preset = match p.preset() {
            "arch" | "viol" | "all" | "custom" => p.preset(),
            _ => "arch",
        };
        let mut m: BTreeMap<&'static str, bool> = LDEF.iter().copied().collect();
        for (k, v) in preset_over(preset) {
            m.insert(k, *v);
        }
        for t in p.lay.split(',').filter(|t| t.len() > 1) {
            let (sign, key) = t.split_at(1);
            if let Some((k, _)) = LDEF.iter().find(|(k, _)| *k == key) {
                match sign {
                    "+" => m.insert(k, true),
                    "-" => m.insert(k, false),
                    _ => None,
                };
            }
        }
        Lay(m)
    }
    pub fn on(&self, key: &str) -> bool {
        self.0.get(key).copied().unwrap_or(true)
    }
    /// `layStr`: the differences from the preset's layers.
    pub fn diff(&self, preset: &str) -> String {
        let mut base: BTreeMap<&str, bool> = LDEF.iter().copied().collect();
        for (k, v) in preset_over(preset) {
            base.insert(k, *v);
        }
        LDEF.iter()
            .filter(|(k, _)| self.on(k) != base[k])
            .map(|(k, _)| format!("{}{k}", if self.on(k) { '+' } else { '-' }))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// `setPreset`: switch preset, dropping the layer differences.
pub fn with_preset(p: &Params, preset: &str) -> Params {
    p.with(|p| {
        p.p = preset.to_string();
        p.lay.clear();
    })
}

/// A toggle click: the layer flips and the preset becomes `custom` (as in the original).
pub fn with_toggle(p: &Params, key: &str) -> Params {
    let mut lay = Lay::from_params(p);
    let v = !lay.on(key);
    if let Some((k, _)) = LDEF.iter().find(|(k, _)| *k == key) {
        lay.0.insert(k, v);
    }
    p.with(|p| {
        p.p = "custom".into();
        p.lay = lay.diff("custom");
    })
}

/// `fOn`: the finding's group and sub-layer are both on.
pub fn f_on(lay: &Lay, code: &str) -> bool {
    let (g, sub) = group_sub(code);
    lay.on(g) && lay.on(&sub)
}

/// `vOn`: `fOn` plus, in the Violations preset, error level only (unless a level is chosen)
/// and the violations scope.
pub fn v_on(lay: &Lay, p: &Params, code: &str, level: &str) -> bool {
    if !f_on(lay, code) {
        return false;
    }
    if p.preset() != "viol" {
        return true;
    }
    let lev_ok = if p.level.is_empty() {
        level == "error"
    } else {
        level == p.level
    };
    lev_ok && (p.vscope() == "all" || group_sub(code).0 == p.vscope())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_round_trip_through_the_url() {
        let p = Params::from_query("?p=viol&lay=%2Bedges,-d-cycle");
        let lay = Lay::from_params(&p);
        assert!(lay.on("edges") && lay.on("cx") && !lay.on("d-cycle"));
        assert_eq!(lay.diff("viol"), "+edges,-d-cycle");
        let all = Params::default().with(|p| p.p = "all".into());
        assert!(Lay::from_params(&all).on("deps"));
        let arch = Lay::from_params(&Params::default());
        assert!(arch.on("deps") && !arch.on("cx") && !arch.on("stale"));
        let t = with_toggle(&Params::default(), "tests");
        assert_eq!(
            (t.p.as_str(), t.lay.as_str()),
            ("custom", "+tests,-cx,-stale,-iface")
        );
    }

    #[test]
    fn violations_preset_shows_errors_only_unless_a_level_is_chosen() {
        let p = Params::default().with(|p| p.p = "viol".into());
        let lay = Lay::from_params(&p);
        assert!(v_on(&lay, &p, "deps/deny", "error"));
        assert!(!v_on(&lay, &p, "stale/file", "warn"));
        let w = p.with(|p| p.level = "warn".into());
        assert!(v_on(&lay, &w, "stale/file", "warn"));
        let scoped = p.with(|p| p.vs = "cx".into());
        assert!(!v_on(&lay, &scoped, "deps/deny", "error"));
    }

    #[test]
    fn codes_map_to_groups() {
        assert_eq!(group_sub("deps/cycle"), ("deps", "d-cycle".into()));
        assert_eq!(group_sub("complexity/nesting"), ("cx", "c-nesting".into()));
        assert_eq!(group_sub("bricks/unmapped"), ("stale", "s-unmapped".into()));
    }
}
