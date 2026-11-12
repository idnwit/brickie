//! Iframe messages (`postMessage`, JSON with a `type` tag) between the host page and the viewer.

use crate::Span;
use serde::{Deserialize, Serialize};

/// Host page -> viewer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToViewer {
    /// Open `path` in a tab, revealing `span` when given.
    Open {
        path: String,
        span: Option<Span>,
    },
    /// Scope the explorer to the node `node` (an id) of view `view`; `None` shows everything.
    Filter {
        view: String,
        node: Option<String>,
    },
    Theme {
        dark: bool,
    },
}

/// Viewer -> host page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToHost {
    /// The viewer listens for messages now (sent once, after it started).
    Ready,
    Opened {
        path: String,
    },
    /// A mark's link was clicked.
    Link {
        href: String,
    },
    Close,
}
