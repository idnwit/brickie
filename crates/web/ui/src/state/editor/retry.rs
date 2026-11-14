//! What a refused write does next: a `design/stale` answer (another writer made a version
//! first) is retried once on the new head; anything else, or a second refusal, is shown as one
//! line.

/// The next step after a write failed with `err` (`"409: <text>"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Reload the design and send the same request on its head.
    Retry,
    /// Show this line.
    Show(String),
}

/// Whether the error is `design/stale`.
pub fn is_stale(err: &str) -> bool {
    err.contains("design/stale")
}

/// The first line of an error, without the HTTP status and the `error[code]: ` prefix.
pub fn one_line(err: &str) -> String {
    let line = err.lines().next().unwrap_or_default();
    let line = match line.split_once(": ") {
        Some((status, rest)) if status.chars().all(|c| c.is_ascii_digit()) => rest,
        _ => line,
    };
    let line = match line
        .strip_prefix("error[")
        .and_then(|r| r.split_once("]: "))
    {
        Some((_, rest)) => rest,
        None => line,
    };
    line.trim().to_string()
}

/// What to do about `err`; `retry`: no retry was made yet.
pub fn on_refused(err: &str, retry: bool) -> Refusal {
    match retry && is_stale(err) {
        true => Refusal::Retry,
        false => Refusal::Show(format!("Not applied: {}", one_line(err))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STALE: &str = "409: error[design/stale]: design is at v5 (by agent, 3s ago), your edit was based on v4. Nothing was written.\nChanges since v4:\n  v5 by agent:\n    + brick tel";

    #[test]
    fn a_stale_write_is_retried_once_then_shown() {
        assert_eq!(on_refused(STALE, true), Refusal::Retry);
        assert_eq!(
            on_refused(STALE, false),
            Refusal::Show(
                "Not applied: design is at v5 (by agent, 3s ago), your edit was based on v4. Nothing was written."
                    .into()
            )
        );
        let gone = "409: remove_rule on brickie.toml: design/no-match: no rule a -> b";
        assert_eq!(
            on_refused(gone, true),
            Refusal::Show(
                "Not applied: remove_rule on brickie.toml: design/no-match: no rule a -> b".into()
            )
        );
        assert_eq!(one_line("request failed"), "request failed");
    }
}
