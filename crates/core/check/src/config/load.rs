//! Loading the config tree: each `brickie.toml` with its `include`s merged is one level; a
//! brick with `path = "<dir>"` loads `<dir>/brickie.toml` as the level below it.

use super::MeasureCfg;
use super::source::{Disk, FileSource};
use super::validate::{validate, validate_via};
use super::{Action, Brick, ComplexityRule, Config, DepRule, Export};
use anyhow::{Context, Result, bail};
use brickie_model::codes::CONFIG_MISSING;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawDeps {
    default: Option<Action>,
    cycles: Option<Action>,
    #[serde(default)]
    rule: Vec<DepRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exports: Vec<Export>,
    #[serde(default)]
    brick: Vec<Brick>,
    #[serde(default)]
    deps: RawDeps,
    #[serde(default)]
    complexity: Vec<ComplexityRule>,
    #[serde(default)]
    measure: Option<MeasureCfg>,
}

/// Line numbers (1-based) of the `[[header]]` table starts, in file order.
fn header_lines(text: &str, header: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.trim() == header)
        .map(|(i, _)| i + 1)
        .collect()
}

/// Load the root config and every composite below it.
pub fn load(path: &Path) -> Result<Config> {
    load_from(&Disk, path)
}

/// `load` over another file source (a change's preview).
pub fn load_from(src: &dyn FileSource, path: &Path) -> Result<Config> {
    load_level(src, path, Vec::new(), &mut Vec::new(), false)
}

/// Only the `[measure]` table of the root config (and its includes): `measure` needs it even
/// when the brick tree is incomplete, so bricks are not resolved or validated. A missing file
/// gives the defaults.
pub fn load_measure(path: &Path) -> Result<MeasureCfg> {
    if !path.exists() {
        return Ok(MeasureCfg::default());
    }
    Ok(load_level(&Disk, path, Vec::new(), &mut Vec::new(), true)?.measure)
}

/// One level: the file and its includes, validated, then the levels of its composites.
/// `ancestors` holds the canonical config files above, to reject a `path` cycle.
fn load_level(
    src: &dyn FileSource,
    path: &Path,
    level: Vec<String>,
    ancestors: &mut Vec<PathBuf>,
    measure_only: bool,
) -> Result<Config> {
    let mut cfg = Config {
        path: level,
        file: path.display().to_string(),
        ..Config::default()
    };
    let canon = src
        .canonicalize(path)
        .with_context(|| format!("read {}", path.display()))?;
    if ancestors.contains(&canon) {
        bail!(
            "brick path cycle: {} contains itself (level {})",
            path.display(),
            cfg.level()
        );
    }
    load_file(src, path, &mut cfg, &mut Vec::new(), &mut BTreeSet::new())?;
    if !cfg.path.is_empty() && cfg.measure != MeasureCfg::default() {
        bail!(
            "{}: [measure] is only allowed in the root brickie.toml",
            cfg.file
        );
    }
    cfg.measure.normalise()?;
    if measure_only {
        return Ok(cfg);
    }
    validate(&cfg)?;
    ancestors.push(canon);
    for b in &mut cfg.bricks {
        let Some(dir) = b.path.clone() else { continue };
        let file = b.base.join(&dir).join("brickie.toml");
        if !src.is_file(&file) {
            bail!(
                "{CONFIG_MISSING}: brick '{}' (level {}, {}) has path = \"{dir}\" but {} does not exist",
                b.name,
                cfg_level(&cfg.path),
                cfg.file,
                file.display()
            );
        }
        if b.members.is_empty() {
            b.members.push(format!("path:{dir}/"));
        }
        let mut below = cfg.path.clone();
        below.push(b.name.clone());
        let sub = load_level(src, &file, below, ancestors, false)
            .with_context(|| format!("brick '{}' in {}", b.name, cfg.file))?;
        b.sub = Some(Box::new(sub));
    }
    ancestors.pop();
    validate_via(&cfg)?;
    Ok(cfg)
}

fn cfg_level(path: &[String]) -> String {
    super::level_name(path)
}

/// Includes are merged before the including file: lists are concatenated, and `default` /
/// `cycles` set by the including file override those of its includes.
fn load_file(
    src: &dyn FileSource,
    path: &Path,
    cfg: &mut Config,
    stack: &mut Vec<PathBuf>,
    done: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let canon = src
        .canonicalize(path)
        .with_context(|| format!("read {}", path.display()))?;
    if stack.contains(&canon) {
        bail!("include cycle: {} includes itself", path.display());
    }
    if !done.insert(canon.clone()) {
        return Ok(()); // already merged through another include
    }
    let text = src
        .read(path)
        .with_context(|| format!("read {}", path.display()))?;
    let raw: Raw = toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    let base = canon.parent().map(Path::to_path_buf).unwrap_or_default();
    stack.push(canon);
    let dir = path.parent().unwrap_or(Path::new("."));
    for inc in &raw.include {
        load_file(src, &dir.join(inc), cfg, stack, done)
            .with_context(|| format!("included from {}", path.display()))?;
    }
    stack.pop();
    for mut b in raw.brick {
        if cfg.bricks.iter().any(|x| x.name == b.name) {
            bail!("duplicate brick '{}' (in {})", b.name, path.display());
        }
        b.base = base.clone();
        cfg.bricks.push(b);
    }
    if let Some(m) = raw.measure {
        cfg.measure.additional_targets.extend(m.additional_targets);
    }
    cfg.exports.extend(raw.exports);
    if let Some(d) = raw.deps.default {
        cfg.deps.default = d;
    }
    if let Some(c) = raw.deps.cycles {
        cfg.deps.cycles = c;
    }
    let loc = |lines: &[usize], i: usize| {
        format!("{}:{}", path.display(), lines.get(i).copied().unwrap_or(0))
    };
    let lines = header_lines(&text, "[[deps.rule]]");
    for (i, mut r) in raw.deps.rule.into_iter().enumerate() {
        r.loc = loc(&lines, i);
        cfg.deps.rules.push(r);
    }
    let lines = header_lines(&text, "[[complexity]]");
    for (i, mut r) in raw.complexity.into_iter().enumerate() {
        r.loc = loc(&lines, i);
        cfg.complexity.push(r);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    fn load_str(text: &str) -> Result<Config> {
        let dir = tempfile::tempdir().unwrap();
        load(&write(dir.path(), "brickie.toml", text))
    }

    #[test]
    fn parses_and_rejects() {
        let c =
            load_str("[deps]\ndefault=\"deny\"\n[[brick]]\nname=\"a\"\nmembers=[\"x\"]\n").unwrap();
        assert_eq!(c.bricks.len(), 1);
        assert_eq!(c.deps.default, Action::Deny);
        let dup = "[[brick]]\nname=\"a\"\nmembers=[]\n[[brick]]\nname=\"a\"\nmembers=[]\n";
        assert!(
            load_str(dup)
                .unwrap_err()
                .to_string()
                .contains("duplicate brick")
        );
        assert!(load_str("bogus = 1\n").is_err());
    }

    #[test]
    fn loads_composites_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let root = write(
            dir.path(),
            "brickie.toml",
            "[[brick]]\nname=\"web\"\npath=\"crates/web\"\n",
        );
        write(
            dir.path(),
            "crates/web/brickie.toml",
            "exports=[\"api\", { brick = \"ui\", scope = \"test\" }]\n\
             [[brick]]\nname=\"api\"\nmembers=[\"web-api\"]\n\
             [[brick]]\nname=\"ui\"\nmembers=[\"web-ui\"]\n",
        );
        let c = load(&root).unwrap();
        let web = c.brick("web").unwrap();
        assert_eq!(web.members, vec!["path:crates/web/".to_string()]);
        let sub = c.node(&["web".into()]).unwrap();
        assert_eq!(sub.level(), "root/web");
        assert!(sub.exports_to("api", "prod"));
        assert!(!sub.exports_to("ui", "prod"));
        assert!(sub.exports_to("ui", "test"));
        // A missing composite config and an unknown export are errors.
        write(
            dir.path(),
            "crates/web/brickie.toml",
            "exports=[\"ghost\"]\n[[brick]]\nname=\"api\"\n",
        );
        let e = format!("{:#}", load(&root).unwrap_err());
        assert!(e.contains("config/unknown-export"), "{e}");
        write(
            dir.path(),
            "brickie.toml",
            "[[brick]]\nname=\"x\"\npath=\"nowhere\"\n",
        );
        let e = format!("{:#}", load(&root).unwrap_err());
        assert!(e.contains("config/missing"), "{e}");
        // A path cycle.
        write(
            dir.path(),
            "brickie.toml",
            "[[brick]]\nname=\"x\"\npath=\".\"\n",
        );
        let e = format!("{:#}", load(&root).unwrap_err());
        assert!(e.contains("cycle"), "{e}");
    }
}
