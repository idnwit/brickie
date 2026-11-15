//! The baseline: the findings of a legacy codebase (or of the start of an agent session) that
//! `check --new` does not report again unless they got worse, plus the hashes of the rule files
//! so a change of the rules is noticed.

use super::Loaded;
use crate::config::{self, Disk, FileSource};
use crate::review;
use anyhow::{Context, Result, bail};
use brickie_facts::sha256_hex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;

/// One finding as saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFinding {
    pub code: String,
    pub weight: u64,
    pub message: String,
}

/// The content of `.brickie/baseline.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    /// Unix seconds.
    pub saved_at: u64,
    /// `git rev-parse HEAD` when the config is in a git work tree.
    pub commit: Option<String>,
    /// Rule file (relative to the config directory) -> sha256 of its content.
    pub rule_files: BTreeMap<String, String>,
    /// Finding id -> the finding when saved. Never holds `stale/*`.
    pub findings: BTreeMap<String, BaselineFinding>,
}

/// Records every file the loader reads.
struct Recording(RefCell<Vec<PathBuf>>);

impl FileSource for Recording {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        self.0.borrow_mut().push(path.to_path_buf());
        Disk.read(path)
    }
    fn is_file(&self, path: &Path) -> bool {
        Disk.is_file(path)
    }
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        Disk.canonicalize(path)
    }
}

fn rel_name(dir: &Path, file: &Path) -> String {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    file.strip_prefix(&dir)
        .unwrap_or(&file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Hash of every `brickie.toml` in the loaded tree (includes too) and of the review rule
/// file `brickie.review.toml`; an absent file hashes as empty.
fn rule_hashes(config: &Path) -> Result<BTreeMap<String, String>> {
    let rec = Recording(RefCell::new(Vec::new()));
    config::load_from(&rec, config)?;
    let dir = review::config_dir(config);
    let mut out = BTreeMap::new();
    for f in rec.0.into_inner() {
        let bytes = std::fs::read(&f).unwrap_or_default();
        out.insert(rel_name(&dir, &f), sha256_hex(&bytes));
    }
    // A fixed name: the review file may be absent, and an absent path does not canonicalize.
    let review = std::fs::read(review::review_path(config)).unwrap_or_default();
    out.insert("brickie.review.toml".into(), sha256_hex(&review));
    Ok(out)
}

fn git_head(dir: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    let head = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !head.is_empty()).then_some(head)
}

impl Baseline {
    /// `<config dir>/.brickie/baseline.json`, next to the cache.
    fn path(config: &Path) -> PathBuf {
        review::config_dir(config).join(".brickie/baseline.json")
    }

    /// The baseline of the current state: every unaccepted finding except `stale/*`.
    pub fn capture(l: &Loaded, config: &Path) -> Result<Baseline> {
        let findings = (l.diagnostics.iter())
            .filter(|d| !d.accepted && !d.code.starts_with("stale/"))
            .map(|d| {
                let weight = l.weights.get(&d.id).copied().unwrap_or(1);
                let f = BaselineFinding {
                    code: d.code.clone(),
                    weight,
                    message: d.message.clone(),
                };
                (d.id.clone(), f)
            })
            .collect();
        let saved_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Ok(Baseline {
            version: VERSION,
            saved_at,
            commit: git_head(&review::config_dir(config)),
            rule_files: rule_hashes(config)?,
            findings,
        })
    }

    /// Writes the file through a temporary file and a rename; returns its path.
    pub fn write(&self, config: &Path) -> Result<PathBuf> {
        let path = Self::path(config);
        let tmp = path.with_extension("json.tmp");
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("write {}", path.display()))?;
        Ok(path)
    }

    pub fn read(config: &Path) -> Result<Baseline> {
        let path = Self::path(config);
        let hint = "run `brickie check --save-baseline` first";
        let bytes = std::fs::read(&path)
            .with_context(|| format!("--new needs a baseline: read {} ({hint})", path.display()))?;
        let b: Baseline = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse {} ({hint})", path.display()))?;
        if b.version != VERSION {
            bail!("{} has version {}; {hint}", path.display(), b.version);
        }
        Ok(b)
    }

    /// Rule files whose hash differs now (changed, added or removed), sorted.
    pub(super) fn changed_rule_files(&self, config: &Path) -> Result<Vec<String>> {
        let now = rule_hashes(config)?;
        let mut names: Vec<&String> = now.keys().chain(self.rule_files.keys()).collect();
        names.sort();
        names.dedup();
        Ok((names.into_iter())
            .filter(|n| now.get(*n) != self.rule_files.get(*n))
            .cloned()
            .collect())
    }
}
