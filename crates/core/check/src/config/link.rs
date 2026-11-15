//! The unused allow rule as the design todo list and the viewer read it.

use super::Scope;

/// An allow rule between two named bricks that no measured edge backs.
#[derive(Debug, Clone)]
pub struct Link {
    /// Names from the root to the level the rule is declared in.
    pub level: Vec<String>,
    pub from: String,
    pub to: String,
    pub via: Option<Vec<String>>,
    /// Empty means all kinds.
    pub kinds: Vec<String>,
    pub scope: Scope,
    /// The config file of the level and `file:line` of the rule.
    pub file: String,
    pub loc: String,
    pub reason: Option<String>,
}

impl Link {
    /// ` no `uses` edge in test scope`: what the rule looks for.
    pub fn filter_text(&self) -> String {
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
        format!("no {kinds} in {scope}")
    }
}
