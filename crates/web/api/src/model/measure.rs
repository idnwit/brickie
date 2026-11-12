//! The server's measure job (`/api/measure`) and how fresh the cache is.

use serde::{Deserialize, Serialize};

/// The state of the server's single measure job. Times are Unix seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MeasureJob {
    /// No measure has run since the server started.
    Idle,
    /// `step` is the step in progress (`cargo metadata`, `rust-analyzer scip`, ...).
    Running {
        step: String,
        started: u64,
    },
    Done {
        at: u64,
    },
    /// `log` is the tail of the job's log (steps, then the error chain).
    Failed {
        error: String,
        log: Vec<String>,
    },
}

impl MeasureJob {
    pub fn running(&self) -> bool {
        matches!(self, MeasureJob::Running { .. })
    }
}

/// `n` seconds as a short age: `just now`, `4m ago`, `2h ago`, `3d ago`.
pub fn age(n: u64) -> String {
    match n {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", n / 60),
        3600..86_400 => format!("{}h ago", n / 3600),
        _ => format!("{}d ago", n / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_json_and_ages() {
        let j = MeasureJob::Running {
            step: "read index".into(),
            started: 5,
        };
        let t = serde_json::to_string(&j).unwrap();
        assert_eq!(t, r#"{"state":"running","step":"read index","started":5}"#);
        assert_eq!(serde_json::from_str::<MeasureJob>(&t).unwrap(), j);
        assert_eq!(
            serde_json::to_string(&MeasureJob::Idle).unwrap(),
            r#"{"state":"idle"}"#
        );
        assert_eq!(
            (age(5), age(250), age(7300)),
            ("just now".into(), "4m ago".into(), "2h ago".into())
        );
    }
}
