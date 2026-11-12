//! The view history the Back / Forward controls walk: the URLs (params) the page pushed, kept in
//! step with the browser's history (`popstate` tells which way it went).

use brickie_web_api::Params;

/// URLs pushed since the page loaded and the one on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nav {
    urls: Vec<String>,
    at: usize,
}

impl Nav {
    pub fn new(url: String) -> Nav {
        Nav {
            urls: vec![url],
            at: 0,
        }
    }

    /// The page pushed `url`: forward entries are dropped.
    pub fn push(&mut self, url: String) {
        self.urls.truncate(self.at + 1);
        self.urls.push(url);
        self.at = self.urls.len() - 1;
    }

    /// The browser went back or forward to `url` (an entry from before the page loaded
    /// starts a new history).
    pub fn popped(&mut self, url: &str) {
        if self.back() == Some(url) {
            self.at -= 1;
        } else if self.forward() == Some(url) {
            self.at += 1;
        } else {
            *self = Nav::new(url.to_string());
        }
    }

    pub fn back(&self) -> Option<&str> {
        self.at.checked_sub(1).map(|i| self.urls[i].as_str())
    }

    pub fn forward(&self) -> Option<&str> {
        self.urls.get(self.at + 1).map(String::as_str)
    }
}

/// What an entry shows: its level, and the selected brick if any (`root/core · api`).
pub fn nav_label(url: &str) -> String {
    let q = url.split_once('?').map_or("", |(_, q)| q);
    let p = Params::from_query(q);
    match p.brick.as_str() {
        "" => p.level_path().to_string(),
        b => format!("{} \u{00B7} {b}", p.level_path()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forward_follow_the_browser() {
        let mut n = Nav::new("/?".into());
        assert_eq!((n.back(), n.forward()), (None, None));
        n.push("/?lv=root/core".into());
        n.push("/?lv=root/core&brick=api".into());
        assert_eq!(n.back(), Some("/?lv=root/core"));
        n.popped("/?lv=root/core");
        assert_eq!(n.forward(), Some("/?lv=root/core&brick=api"));
        n.popped("/?lv=root/core&brick=api");
        assert_eq!(n.forward(), None);
        n.popped("/?lv=root/core");
        n.push("/?lv=root/web".into());
        assert_eq!(n.forward(), None);
        assert_eq!(n.back(), Some("/?lv=root/core"));
        n.popped("/elsewhere");
        assert_eq!((n.back(), n.forward()), (None, None));
    }

    #[test]
    fn labels_name_the_level_and_brick() {
        assert_eq!(nav_label("/"), "root");
        assert_eq!(
            nav_label("/?lv=root/core&brick=api"),
            "root/core \u{00B7} api"
        );
    }
}
