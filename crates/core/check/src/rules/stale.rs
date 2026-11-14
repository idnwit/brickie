//! Staleness: files that changed since `measure`. Config edits never make the cache stale:
//! bricks are resolved from the configs on every run.
//!
//! Candidates come from git (`diff --name-only <cached commit>` plus `status --porcelain`) and
//! are confirmed by sha256. Without git, or when the cached commit is gone, every cached file
//! is hashed and the crate source directories are walked for added files. A cache measured on a
//! dirty tree also hashes every cached file: a file edited before measure and reverted to HEAD
//! afterwards is invisible to git. `check` never re-measures.

use crate::config::{Level, MeasureCfg};
use crate::diag::{Finding, RuleRef, Span};
use brickie_facts::sha256_hex;
use brickie_facts::{Cache, Measured};
use brickie_model::codes::{STALE_FILE, STALE_MEASURE};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy)]
enum Change {
    Changed,
    Added,
    Deleted,
}

struct Stale {
    file: String,
    change: Change,
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// `.rs` paths relative to `root` that git considers different from the cached commit.
fn git_candidates(root: &Path, commit: &str) -> Option<BTreeSet<String>> {
    git(root, &["cat-file", "-e", &format!("{commit}^{{commit}}")])?;
    let mut set = BTreeSet::new();
    let diff = git(root, &["diff", "--name-only", "--relative", "-z", commit])?;
    set.extend(diff.split('\0').map(str::to_string));
    // Porcelain paths are relative to the repository root, so strip the workspace prefix.
    let prefix = git(root, &["rev-parse", "--show-prefix"])?
        .trim()
        .to_string();
    let status = git(root, &["status", "--porcelain", "-z", "-uall"])?;
    let mut it = status.split('\0');
    while let Some(entry) = it.next() {
        if entry.len() < 4 {
            continue;
        }
        let (xy, path) = entry.split_at(3);
        if xy.contains(['R', 'C']) {
            // A rename entry is followed by its original path.
            set.extend(it.next().map(str::to_string));
        }
        set.insert(path.to_string());
    }
    Some(
        set.into_iter()
            .filter_map(|p| match p.strip_prefix(&prefix) {
                Some(rel) => Some(rel.to_string()),
                None if prefix.is_empty() => Some(p),
                None => None,
            })
            .filter(|p| p.ends_with(".rs"))
            .collect(),
    )
}

pub fn evaluate(cache: &Cache) -> Vec<Box<dyn Finding>> {
    let root = Path::new(&cache.workspace_root);
    let mut out: Vec<Box<dyn Finding>> = Vec::new();
    // Directories holding crate roots: an untracked `.rs` file elsewhere is not part of the
    // measured workspace.
    let crate_dirs = brickie_facts::crate_dirs(&cache.crates);
    let in_crate = |p: &str| {
        crate_dirs
            .iter()
            .any(|d| d.is_empty() || p.starts_with(&format!("{d}/")))
    };
    let git_set = cache
        .git
        .as_ref()
        .and_then(|g| git_candidates(root, &g.commit));
    let mut candidates: BTreeSet<String> = match &git_set {
        Some(set) => set.iter().filter(|p| in_crate(p)).cloned().collect(),
        None => {
            let mut all = BTreeSet::new();
            for d in &crate_dirs {
                brickie_facts::walk_rs(&root.join(d), root, &mut all);
            }
            all
        }
    };
    if git_set.is_none() || cache.git.as_ref().is_some_and(|g| g.dirty) {
        candidates.extend(cache.files.keys().cloned());
    }
    for path in candidates {
        let now = std::fs::read(root.join(&path)).ok();
        let change = match (cache.files.get(&path), now) {
            (Some(_), None) => Change::Deleted,
            (Some(f), Some(b)) if sha256_hex(&b) != f.sha256 => Change::Changed,
            (None, Some(_)) => Change::Added,
            _ => continue,
        };
        out.push(Box::new(Stale { file: path, change }));
    }
    out
}

const HELP: &str = "run `brickie measure` to refresh the cache";

impl Finding for Stale {
    fn code(&self) -> String {
        STALE_FILE.into()
    }
    fn level(&self) -> Level {
        Level::Warn
    }
    fn subject(&self) -> String {
        self.file.clone()
    }
    fn message(&self) -> String {
        let what = match self.change {
            Change::Changed => "changed",
            Change::Added => "added",
            Change::Deleted => "deleted",
        };
        format!("`{}` was {what} after it was measured", self.file)
    }
    fn spans(&self) -> Vec<Span> {
        match self.change {
            Change::Deleted => vec![],
            _ => vec![Span {
                file: self.file.clone(),
                line: 0,
                col: None,
                len: None,
                label: None,
            }],
        }
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        Some(HELP.into())
    }
}

/// The settings `measure` runs with for `cfg`: all features plus the additional targets.
pub fn measured_for(cfg: &MeasureCfg) -> Measured {
    Measured::all(cfg.additional_targets.clone())
}

/// A cache measured with other cargo settings than the config asks for. A cache that does not
/// record its settings (empty `features`) is not judged.
pub fn evaluate_measure(cache: &Cache, cfg: &MeasureCfg) -> Vec<Box<dyn Finding>> {
    let want = measured_for(cfg);
    if cache.measured.features.is_empty() || cache.measured == want {
        return Vec::new();
    }
    vec![Box::new(StaleMeasure {
        have: cache.measured.clone(),
        want,
    })]
}

struct StaleMeasure {
    have: Measured,
    want: Measured,
}

impl Finding for StaleMeasure {
    fn code(&self) -> String {
        STALE_MEASURE.into()
    }
    fn level(&self) -> Level {
        Level::Warn
    }
    fn subject(&self) -> String {
        "measure".into()
    }
    fn message(&self) -> String {
        format!(
            "the cache was measured with features `{}` and additional targets {:?}; the config asks for features `{}` and targets {:?}",
            self.have.features, self.have.targets, self.want.features, self.want.targets
        )
    }
    fn spans(&self) -> Vec<Span> {
        vec![]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        Some(HELP.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_settings_mismatch_is_stale() {
        let mut cache = crate::config::tests::cache(Path::new("."), &[]);
        let mut cfg = MeasureCfg::default();
        // A cache without recorded settings is not judged.
        assert!(evaluate_measure(&cache, &cfg).is_empty());
        cache.measured = measured_for(&cfg);
        assert!(evaluate_measure(&cache, &cfg).is_empty());
        cfg.additional_targets = vec!["wasm32-unknown-unknown".into()];
        let f = evaluate_measure(&cache, &cfg);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code(), STALE_MEASURE);
        assert!(f[0].message().contains("wasm32-unknown-unknown"));
        cache.measured = measured_for(&cfg);
        assert!(evaluate_measure(&cache, &cfg).is_empty());
    }
}
