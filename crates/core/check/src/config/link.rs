//! A rule between two named bricks as the design todo list and the viewer read it.

use super::{Action, Scope};

/// A rule between two named bricks: an unused allow rule (no measured edge backs it), or any
/// rule of the design as Design mode draws it.
#[derive(Debug, Clone)]
pub struct Link {
    /// Names from the root to the level the rule is declared in.
    pub level: Vec<String>,
    pub from: String,
    pub to: String,
    /// Exported children of `to` the rule is limited to (empty: any entry).
    pub via: Vec<String>,
    /// Empty means all kinds.
    pub kinds: Vec<String>,
    pub scope: Scope,
    pub action: Action,
    /// The config file of the level and `file:line` of the rule.
    pub file: String,
    pub loc: String,
    pub reason: Option<String>,
}

impl Link {
    /// `no `uses` edge in test scope`: what an unused rule still waits for.
    pub fn filter_text(&self) -> String {
        format!("no {}", self.match_text())
    }

    /// `` `uses` edge in test scope``: what the rule matches.
    pub fn match_text(&self) -> String {
        let kinds = if self.kinds.is_empty() {
            "edge".to_string()
        } else {
            let k: Vec<String> = self.kinds.iter().map(|k| format!("`{k}`")).collect();
            format!("{} edge", k.join(" or "))
        };
        let scope = match self.scope {
            Scope::All => "any scope".to_string(),
            s => format!("{} scope", s.name()),
        };
        let via = if self.via.is_empty() {
            String::new()
        } else {
            let v: Vec<String> = self.via.iter().map(|v| format!("`{v}`")).collect();
            format!(" entering through {}", v.join(" or "))
        };
        format!("{kinds}{via} in {scope}")
    }
}
