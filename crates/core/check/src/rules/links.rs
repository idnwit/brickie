//! The declared links of the design, judged by [`judge_link`] and marked when on a cycle (read
//! from the design files alone), and the valid links no measured edge uses yet ("planned";
//! information, never a finding).

use crate::config::{Config, LinkState, Resolved, covering_link, judge_link, link_cycles};
use brickie_facts::Cache;

/// Every link of the tree, in tree order, judged.
pub fn design_links(cfg: &Config) -> Vec<LinkState> {
    let mut out = Vec::new();
    walk(cfg, &mut out);
    out
}

fn walk(level: &Config, out: &mut Vec<LinkState>) {
    let cycles = link_cycles(level);
    for l in &level.links {
        out.push(LinkState {
            level: level.path.clone(),
            file: level.file.clone(),
            link: l.clone(),
            denied: judge_link(level, l),
            cycle: cycles.iter().any(|c| std::ptr::eq(*c, l)),
            planned: false,
        });
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, out);
        }
    }
}

/// Every link of the tree, in tree order, with `planned` set on the valid ones that no measured
/// edge goes through yet.
pub fn marked_links(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<LinkState> {
    let mut used: Vec<String> = Vec::new();
    for e in cache.edges() {
        let (Some(p1), Some(p2)) = (res.path(&e.from), res.path(&e.to)) else {
            continue;
        };
        let Some(l) = super::lift::lift(cfg, p1, p2) else {
            continue;
        };
        let entered = p2.get(l.k + 1).map(String::as_str);
        if let Ok(Some(link)) = covering_link(l.level, l.from, l.to, entered, &e.kind, &e.scope) {
            used.push(link.loc.clone());
        }
    }
    let mut all = design_links(cfg);
    for s in &mut all {
        s.planned = s.denied.is_none() && !used.contains(&s.link.loc);
    }
    all
}

/// The valid links that no measured edge goes through yet.
pub fn planned_links(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<LinkState> {
    marked_links(cache, cfg, res)
        .into_iter()
        .filter(|s| s.planned)
        .collect()
}
