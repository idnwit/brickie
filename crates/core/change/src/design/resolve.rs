//! Names to files: writers address bricks by path (`web/api`) and levels by the brick that
//! owns them; ops need the physical config file. Read from the files on disk.

use super::Design;
use crate::edit::{Docs, find_rule};
use crate::paths::{join_rel, to_rel};
use anyhow::{Context, Result, bail};
use brickie_check::config::{self, Config};
use brickie_model::codes::DESIGN_NO_MATCH;
use brickie_model::design::{ExportSpec, Op, RuleKey};
use toml_edit::Item;

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
    /// The design on disk, loaded and validated.
    pub fn config(&self) -> Result<Config> {
        config::load(&self.config_path())
    }

    /// The config file of the level below brick `level` (`[]`: the root level).
    pub fn level_file(&self, level: &[String]) -> Result<String> {
        let cfg = self.config()?;
        self.file_of(&cfg, level)
    }

    fn file_of(&self, cfg: &Config, level: &[String]) -> Result<String> {
        let node = cfg.node(level).with_context(|| {
            format!(
                "{DESIGN_NO_MATCH}: no composite brick '{}'",
                level.join("/")
            )
        })?;
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
    pub fn exports(&self, file: &str) -> Result<Vec<ExportSpec>> {
        let docs = Docs::from_base(&self.files()?, None)?;
        let doc = docs
            .get(file)
            .with_context(|| format!("{file} is not in the design"))?;
        let list = doc.get("exports").and_then(Item::as_array);
        Ok(list
            .into_iter()
            .flat_map(|a| a.iter())
            .filter_map(|v| match v {
                toml_edit::Value::String(s) => Some(ExportSpec {
                    brick: s.value().clone(),
                    scope: None,
                }),
                toml_edit::Value::InlineTable(t) => Some(ExportSpec {
                    brick: t.get("brick")?.as_str()?.to_string(),
                    scope: t.get("scope").and_then(|s| s.as_str()).map(String::from),
                }),
                _ => None,
            })
            .collect())
    }

    /// The file holding the rule `key` among the level file and what it includes; the level
    /// file when no file has it (the op then reports the candidates).
    pub fn rule_file(&self, level_file: &str, key: &RuleKey) -> Result<String> {
        let docs = Docs::from_base(&self.files()?, None)?;
        let mut queue = vec![level_file.to_string()];
        let mut seen = Vec::new();
        while let Some(f) = queue.pop() {
            if seen.contains(&f) {
                continue;
            }
            if let Some(doc) = docs.get(&f) {
                let inc = doc.get("include").and_then(Item::as_array);
                for i in inc
                    .into_iter()
                    .flat_map(|a| a.iter())
                    .filter_map(|v| v.as_str())
                {
                    queue.push(join_rel(&f, i));
                }
            }
            seen.push(f);
        }
        let hits: Vec<&String> = (seen.iter())
            .filter(|f| docs.get(f).is_some_and(|d| find_rule(d, f, key).is_ok()))
            .collect();
        match hits.as_slice() {
            [f] => Ok((*f).clone()),
            [] => Ok(level_file.to_string()),
            many => {
                let files: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
                bail!(
                    "{DESIGN_NO_MATCH}: rule {} -> {} is in {}; edit one file directly",
                    key.from,
                    key.to,
                    files.join(" and ")
                )
            }
        }
    }
}
