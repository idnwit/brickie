//! `design show`: the design at the head as a tree, marked with what the code lacks.

use super::todo::todo;
use crate::design::Design;
use crate::paths::to_rel;
use anyhow::Result;
use brickie_check::check;
use brickie_check::config::{self, Action, Brick, Config, Scope, level_name};
use brickie_model::design::{BrickView, DesignView, LevelView, LimitView, RuleView};
use std::collections::BTreeMap;
use std::path::Path;

struct Marks<'a> {
    root: &'a Path,
    planned: Vec<String>,
    unbacked: Vec<String>,
}

impl Design {
    /// The design at the head; fails when it does not load.
    pub fn show(&self) -> Result<DesignView> {
        let head = self.head()?;
        let cfg = self.config()?;
        let loaded = match self.root().join(".brickie/cache.json").is_file() {
            true => check::load_draft_from(&config::Disk, &self.config_path())
                .map_err(|e| format!("{e:#}")),
            false => {
                Err("no measurements yet: run `brickie measure` to list what the code lacks".into())
            }
        };
        let t = todo(self.root(), &cfg, loaded.as_ref().map_err(String::clone));
        let marks = Marks {
            root: self.root(),
            planned: t.planned.iter().map(|p| level_name(p)).collect(),
            unbacked: t.unbacked,
        };
        Ok(DesignView {
            v: head.v,
            hash: head.hash.clone(),
            by: head.by,
            at: head.at,
            root: level(&cfg, &marks),
            todo: t.tasks,
        })
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

fn action(a: Action) -> String {
    match a {
        Action::Allow => "allow",
        Action::Deny => "deny",
    }
    .into()
}

fn scope(s: Scope) -> String {
    match s {
        Scope::Prod => "prod",
        Scope::Test => "test",
        Scope::All => "all",
    }
    .into()
}

fn level(cfg: &Config, m: &Marks) -> LevelView {
    let rules = cfg.deps.rules.iter().map(|r| RuleView {
        file: rel(m.root, &r.loc),
        from: r.from.to_string(),
        to: r.to.to_string(),
        via: r.via.clone().unwrap_or_default(),
        kinds: r.kinds.clone(),
        scope: scope(r.scope),
        action: action(r.action),
        reason: r.reason.clone().filter(|s| !s.trim().is_empty()),
        todo: m.unbacked.contains(&r.loc),
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
            file: rel(m.root, &c.loc),
            bricks: c.bricks.to_string(),
            metrics,
            level: format!("{:?}", c.level).to_lowercase(),
            scope: scope(c.scope),
        }
    });
    LevelView {
        path: cfg.level(),
        file: rel(m.root, &cfg.file),
        bricks: cfg.bricks.iter().map(|b| brick(cfg, b, m)).collect(),
        exports: cfg.exports.iter().map(|e| e.brick.clone()).collect(),
        default: action(cfg.deps.default),
        cycles: action(cfg.deps.cycles),
        rules: rules.collect(),
        limits: limits.collect(),
    }
}

fn brick(cfg: &Config, b: &Brick, m: &Marks) -> BrickView {
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
        todo: m.planned.contains(&level_name(&path)),
        children: b.sub.as_ref().map(|s| Box::new(level(s, m))),
    }
}
