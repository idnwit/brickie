//! Projection of enclosing-level `[[link]]`s onto a composite level's boundary.

use brickie_check::config::{Config, level_name};
use brickie_web_api::LevelLink;

/// One projected enclosing-level link before ghost/child indices are resolved.
pub(super) struct Projected {
    pub outside: Vec<String>,
    pub inbound: bool,
    pub child: Option<String>,
    pub link: LevelLink,
}

/// Project one ancestor `[[link]]` onto `cfg`'s boundary, if it crosses `cfg`.
fn project_link(
    anc: &[String],
    me: &str,
    next: Option<&str>,
    cfg: &Config,
    link: &LevelLink,
) -> Option<Projected> {
    let (to_brick, to_child) = link
        .to
        .split_once('/')
        .map_or((link.to.as_str(), None), |(b, c)| (b, Some(c)));
    if link.from == me {
        let mut outside = anc.to_vec();
        outside.push(to_brick.to_string());
        return Some(Projected {
            outside,
            inbound: false,
            child: None,
            link: link.clone(),
        });
    }
    if to_brick != me {
        return None;
    }
    if let Some(next_seg) = next
        && to_child.is_some_and(|c| c != next_seg)
    {
        return None;
    }
    let child = next
        .is_none()
        .then(|| {
            to_child
                .filter(|c| cfg.unit_by_name(c).is_some())
                .map(str::to_string)
        })
        .flatten();
    let mut outside = anc.to_vec();
    outside.push(link.from.clone());
    Some(Projected {
        outside,
        inbound: true,
        child,
        link: link.clone(),
    })
}

/// Enclosing-level `[[link]]`s that cross `cfg`'s boundary (from any ancestor level).
pub(super) fn projected_links(
    root: &Config,
    links: &[(String, LevelLink)],
    cfg: &Config,
) -> Vec<Projected> {
    let mut out = Vec::new();
    for k in 0..cfg.path.len() {
        let anc = &cfg.path[..k];
        let anc_name = level_name(anc);
        let me_raw = cfg.path[k].as_str();
        let me = root
            .node(anc)
            .and_then(|c| c.brick(me_raw).map(|b| c.unit_of(b).name))
            .unwrap_or(me_raw);
        let next = cfg.path.get(k + 1).map(|nr| {
            root.node(&cfg.path[..=k])
                .and_then(|c| c.brick(nr).map(|b| c.unit_of(b).name))
                .unwrap_or(nr.as_str())
        });
        for (lv, link) in links {
            if *lv == anc_name
                && let Some(pr) = project_link(anc, me, next, cfg, link)
            {
                out.push(pr);
            }
        }
    }
    out
}
