//! The server's measure job: one background run of `brickie-measure` at a time (single
//! flight). Starting while a run is in progress returns that run. The run holds the cache lock
//! (`brickie_facts::CacheLock`), so it never writes `cache.json` together with a CLI
//! `brickie measure`: whichever comes second fails with a clear message.

use brickie_web_api::MeasureJob;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// Runs one measure of the config, reporting each step's name.
pub(crate) type Runner = Arc<dyn Fn(&Path, &dyn Fn(&str)) -> anyhow::Result<()> + Send + Sync>;

/// Lines of the log kept in a failed job.
const LOG_TAIL: usize = 20;

pub(crate) struct Jobs {
    state: Mutex<MeasureJob>,
    runner: Runner,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The real runner: `brickie measure --config <config>` in-process.
pub(crate) fn measure_runner() -> Runner {
    Arc::new(|config: &Path, progress: &dyn Fn(&str)| {
        let opts = brickie_measure::Options {
            config: config.to_path_buf(),
            scip: None,
            workspace: None,
            additional_targets: brickie_check::config::load_measure(config)?.additional_targets,
        };
        brickie_measure::run_with(&opts, progress).map(|_| ())
    })
}

impl Jobs {
    pub(crate) fn new(runner: Runner) -> Jobs {
        Jobs {
            state: Mutex::new(MeasureJob::Idle),
            runner,
        }
    }

    pub(crate) fn status(&self) -> MeasureJob {
        lock(&self.state).clone()
    }

    /// Start a run of `config` on the blocking pool, or return the run in progress.
    pub(crate) fn start(self: &Arc<Self>, config: PathBuf) -> MeasureJob {
        let started = {
            let mut s = lock(&self.state);
            if s.running() {
                return s.clone();
            }
            *s = MeasureJob::Running {
                step: "starting".into(),
                started: now(),
            };
            s.clone()
        };
        let me = self.clone();
        tokio::task::spawn_blocking(move || me.run(&config));
        started
    }

    fn run(&self, config: &Path) {
        let log = Mutex::new(Vec::new());
        let progress = |step: &str| {
            lock(&log).push(step.to_string());
            if let MeasureJob::Running { step: s, .. } = &mut *lock(&self.state) {
                *s = step.to_string();
            }
        };
        let end = match (self.runner)(config, &progress) {
            Ok(()) => MeasureJob::Done { at: now() },
            Err(e) => {
                let mut log = std::mem::take(&mut *lock(&log));
                log.extend(e.chain().map(|c| format!("error: {c}")));
                let skip = log.len().saturating_sub(LOG_TAIL);
                MeasureJob::Failed {
                    error: format!("{e:#}"),
                    log: log.split_off(skip),
                }
            }
        };
        *lock(&self.state) = end;
    }
}
