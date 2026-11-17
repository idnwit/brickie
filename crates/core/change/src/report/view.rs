//! `design show`: the working tree (the draft, else the published design) as a tree, read from
//! the design files alone.

use super::todo::design_todo;
use crate::design::Design;
use crate::paths::to_rel;
use anyhow::Result;
use brickie_check::config::{Brick, Config, Scope, judge_link, link_cycles};
use brickie_model::design::{
    BrickView, DesignView, LevelView, LimitView, LinkView, RuleView, Task,
};
use std::collections::BTreeMap;
use std::path::Path;

impl Design {
    /// The todo list of a draft held in memory: config self-consistency only.
    pub(crate) fn todo_of(&self, src: &crate::preview::Preview) -> Vec<Task> {
        match src.load_config() {
            Ok(cfg) => design_todo(&cfg),
            Err(_) => Vec::new(),
        }
    }

    /// The config tree for `show`. A missing nested file leaves its brick a leaf and is named in
    /// the warnings, so the rest of the design still shows.
    fn config_lenient(&self) -> Result<(Config, Vec<String>)> {
        let cfg = self.config()?;
        let warnings = match cfg.missing.is_empty() {
            true => Vec::new(),
            false => crate::design::missing(self.root(), &self.files()?),
        };
        let warnings = (warnings.iter())
            .map(|m| {
                format!(
                    "missing file {m}: its brick shows as a leaf; `brickie design undo` restores it"
                )
            })
            .collect();
        Ok((cfg, warnings))
    }

    /// The design at the head from its files alone (no measured code), plus warnings about
    /// config files that are missing. Config tasks only: what the code lacks is `brickie check`'s todo.
    pub fn show_checked(&self) -> Result<(DesignView, Vec<String>)> {
        let head = self.head()?;
        let (cfg, warnings) = self.config_lenient()?;
        let view = DesignView {
            v: head.v,
            track: head.track,
            forked_from: self.forked_from()?,
            hash: head.hash.clone(),
            by: head.by,
            at: head.at,
            root: level(&cfg, self.root()),
            todo: design_todo(&cfg),
        };
        Ok((view, warnings))
    }

    /// [`Design::show_checked`] without the warnings: what the viewer's Design mode shows.
    pub fn show_design(&self) -> Result<DesignView> {
        Ok(self.show_checked()?.0)
    }
}

fn rel(root: &Path, file: &str) -> String {
    let file = file
        .rsplit_once(':')
        .map_or(file, |(f, l)| match l.parse::<u32>() {
            Ok(_) => f,
            Err(_) => file,
        });
    let abs = std::path::absolute(file).unwrap_or_default();
    to_rel(root, &abs).unwrap_or_else(|| file.to_string())
}

fn scope(s: Scope) -> String {
    match s {
        Scope::Prod => "prod",
        Scope::Test => "test",
        Scope::All => "all",
    }
    .into()
}

fn level(cfg: &Config, root: &Path) -> LevelView {
    let rules = cfg.rules.iter().map(|r| RuleView {
        file: rel(root, &r.loc),
        from: r.from.to_string(),
        to: r.to.to_string(),
        kinds: r.kinds.clone(),
        scope: scope(r.scope),
        reason: r.reason.clone(),
    });
    let cycles = link_cycles(cfg);
    let links = cfg.links.iter().map(|l| LinkView {
        file: rel(root, &l.loc),
        from: l.from.clone(),
        to: l.to.clone(),
        kinds: l.kinds.clone(),
        scope: scope(l.scope),
        reason: l.reason.clone(),
        denied: judge_link(cfg, l),
        cycle: cycles.iter().any(|c| std::ptr::eq(*c, l)),
    });
    let limits = cfg.complexity.iter().map(|c| {
        let all = [
            ("cognitive", c.cognitive.map(u64::from)),
            ("cyclomatic", c.cyclomatic.map(u64::from)),
            ("nesting", c.nesting.map(u64::from)),
            ("paths", c.paths),
            ("params", c.params.map(u64::from)),
            ("lines", c.lines.map(u64::from)),
        ];
        let metrics: BTreeMap<String, u64> = (all.into_iter())
            .filter_map(|(k, v)| Some((k.to_string(), v?)))
            .collect();
        LimitView {
            file: rel(root, &c.loc),
            bricks: c.bricks.to_string(),
            metrics,
            scope: scope(c.scope),
        }
    });
    LevelView {
        path: cfg.level(),
        file: rel(root, &cfg.file),
        bricks: cfg.bricks.iter().map(|b| brick(cfg, b, root)).collect(),
        exports: cfg.exports.clone(),
        rules: rules.collect(),
        links: links.collect(),
        limits: limits.collect(),
    }
}

fn brick(cfg: &Config, b: &Brick, root: &Path) -> BrickView {
    let kind = match (&b.sub, b.members.iter().all(|x| x.starts_with("path:"))) {
        (Some(_), _) => "group",
        (None, true) if !b.members.is_empty() => "crate",
        _ => "module",
    };
    let mut path = cfg.path.clone();
    path.push(b.name.clone());
    BrickView {
        name: b.name.clone(),
        kind: kind.into(),
        members: b.members.clone(),
        tags: b.tags.clone(),
        desc: b.desc.clone(),
        independent: b.independent,
        children: b.sub.as_ref().map(|s| Box::new(level(s, root))),
    }
}
