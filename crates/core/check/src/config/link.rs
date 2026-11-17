//! A declared link with what the design says of it: the view `design check`, the design todo
//! list and the viewer read.

use super::{Link, Scope};

/// A `[[link]]` of some level, judged.
#[derive(Debug, Clone)]
pub struct LinkState {
    /// Names from the root to the level the link is declared in.
    pub level: Vec<String>,
    /// The config file of the level.
    pub file: String,
    pub link: Link,
    /// Why no rule allows it (`design/link-denied`); `None` for a valid link.
    pub denied: Option<String>,
    /// The link sits on a cycle of its level's links (information only).
    pub cycle: bool,
    /// Valid, and no measured edge goes through it yet (information only; set only when code
    /// was measured, see `marked_links`).
    pub planned: bool,
}

impl LinkState {
    /// `` `uses` edge in test scope``: what the link covers.
    pub fn match_text(&self) -> String {
        let l = &self.link;
        let kinds = if l.kinds.is_empty() {
            "edge".to_string()
        } else {
            let k: Vec<String> = l.kinds.iter().map(|k| format!("`{k}`")).collect();
            format!("{} edge", k.join(" or "))
        };
        let scope = match l.scope {
            Scope::All => "any scope".to_string(),
            s => format!("{} scope", s.name()),
        };
        format!("{kinds} in {scope}")
    }
}
