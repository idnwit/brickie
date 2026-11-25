//! Names to files: writers address bricks by path (`web/api`) and levels by the brick that
//! owns them; ops need the physical config file. Read from the working tree (the draft, if any).

use super::Design;
use crate::edit::{Docs, find_rule, level_files};
use crate::paths::to_rel;
use anyhow::{Context, Result, bail};
use brickie_check::config::{self, Config};
use brickie_model::codes::DESIGN_NO_MATCH;
use brickie_model::design::{Op, RuleKey};

/// `root/web/api`, `web/api` -> `["web", "api"]`; `root`, `.`, `` -> `[]`.
pub fn brick_path(s: &str) -> Vec<String> {
    let s = s.trim_matches('/');
    let s = s
        .strip_prefix("root")
        .map_or(s, |r| r.trim_start_matches('/'));
    s.split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .map(String::from)
        .collect()
}

impl Design {
    /// The working tree (the draft when one exists, else the published files), loaded and
    /// validated.
    pub fn config(&self) -> Result<Config> {
        match self.draft()? {
            Some(d) => self.preview(&d.files)?.load_config(),
            None => config::load(&self.config_path()),
        }
    }

    /// The config file of the level below brick `level` (`[]`: the root level).
    pub fn level_file(&self, level: &[String]) -> Result<String> {
        let cfg = self.config()?;
        self.file_of(&cfg, level)
    }

    pub(super) fn file_of(&self, cfg: &Config, level: &[String]) -> Result<String> {
        let Some(node) = cfg.node(level) else {
            if let Some((name, parent)) = level.split_last()
                && cfg.node(parent).and_then(|lv| lv.group(name)).is_some()
            {
                bail!(
                    "{DESIGN_NO_MATCH}: '{}' is a group, not a composite brick",
                    level.join("/")
                );
            }
            bail!(
                "{DESIGN_NO_MATCH}: no composite brick '{}'",
                level.join("/")
            );
        };
        to_rel(&self.root, &std::path::absolute(&node.file)?)
            .context("level file outside the workspace")
    }

    /// The file declaring the brick at `path` and its name.
    pub fn brick_file(&self, path: &[String]) -> Result<(String, String)> {
        let cfg = self.config()?;
        let (name, parent) = path.split_last().context("name a brick")?;
        if cfg.brick_at(path).is_none() {
            bail!("{DESIGN_NO_MATCH}: no brick '{}'", path.join("/"));
        }
        Ok((self.file_of(&cfg, parent)?, name.clone()))
    }

    /// The file declaring the group at `path` (which may be an `include`d file) and its name.
    pub fn group_file(&self, path: &[String]) -> Result<(String, String)> {
        let cfg = self.config()?;
        let (name, parent) = path.split_last().context("name a group")?;
        let lv = cfg.node(parent).with_context(|| {
            format!(
                "{DESIGN_NO_MATCH}: no composite brick '{}'",
                parent.join("/")
            )
        })?;
        let Some(g) = lv.group(name) else {
            bail!("{DESIGN_NO_MATCH}: no group '{}'", path.join("/"));
        };
        let loc_file = g.loc.rsplit_once(':').map_or(g.loc.as_str(), |(f, _)| f);
        if !loc_file.is_empty()
            && let Ok(abs) = std::path::absolute(loc_file)
            && let Some(rel) = to_rel(&self.root, &abs)
        {
            return Ok((rel, name.clone()));
        }
        Ok((self.file_of(&cfg, parent)?, name.clone()))
    }

    /// The level file owning the ungrouped brick or group at `path`, and its name.
    pub fn unit_file(&self, path: &[String]) -> Result<(String, String)> {
        let cfg = self.config()?;
        let (name, parent) = path.split_last().context("name a brick or group")?;
        let lv = cfg.node(parent).with_context(|| {
            format!(
                "{DESIGN_NO_MATCH}: no composite brick '{}'",
                parent.join("/")
            )
        })?;
        if lv.unit_by_name(name).is_none() {
            if let Some(b) = lv.brick(name)
                && let Some(g) = &b.group
            {
                bail!(
                    "{DESIGN_NO_MATCH}: '{}' is in group '{g}'; target the group instead",
                    path.join("/")
                );
            }
            bail!("{DESIGN_NO_MATCH}: no brick or group '{}'", path.join("/"));
        }
        Ok((self.file_of(&cfg, parent)?, name.clone()))
    }

    /// `move_brick` for the brick at `from` into the brick at `into` (`[]`: the root level).
    pub fn move_op(&self, from: &[String], into: &[String]) -> Result<Op> {
        let cfg = self.config()?;
        let (file, name) = self.brick_file(from)?;
        let (into_file, into) = match into.split_last() {
            None => (self.config.clone(), None),
            Some((p, above)) => (self.file_of(&cfg, above)?, Some(p.clone())),
        };
        Ok(Op::MoveBrick {
            file,
            name,
            into_file,
            into,
            path: None,
        })
    }

    /// The exports of a level file as written.
    pub fn exports(&self, file: &str) -> Result<Vec<String>> {
        let docs = Docs::from_base(&self.files()?, None)?;
        let doc = docs
            .get(file)
            .with_context(|| format!("{file} is not in the design"))?;
        Ok(crate::edit::read_exports(doc))
    }

    /// The file holding the rule (or, with `link`, the link) `key` among the level file and
    /// what it includes; the level file when no file has it (the op then reports the
    /// candidates).
    pub fn rule_file(&self, level_file: &str, link: bool, key: &RuleKey) -> Result<String> {
        let table: &[&str] = if link { &["link"] } else { &["rule"] };
        let docs = Docs::from_base(&self.files()?, None)?;
        let files = level_files(&docs, level_file);
        let hits: Vec<&String> = files
            .iter()
            .filter(|f| {
                docs.get(f)
                    .is_some_and(|d| find_rule(d, table, f, key).is_ok())
            })
            .collect();
        match hits.as_slice() {
            [f] => Ok((*f).clone()),
            [] => Ok(level_file.to_string()),
            many => {
                let files: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
                bail!(
                    "{DESIGN_NO_MATCH}: {} {} -> {} is in {}; edit one file directly",
                    table[0],
                    key.from,
                    key.to,
                    files.join(" and ")
                )
            }
        }
    }
}
