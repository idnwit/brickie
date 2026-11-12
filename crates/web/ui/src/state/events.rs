//! What the page does with a Server-Sent Event (`/api/events`): refresh the open change in
//! place, follow the measure job, or show a "refresh" pill when the page data (cache, config)
//! changed under it. The first `cache` / `config` event of a connection is the baseline.

use brickie_web_api::{MeasureJob, ServerEvent};
use sycamore::prelude::*;

/// Signals the event stream drives, shared by every level.
#[derive(Clone, Copy)]
pub struct Bus {
    /// A notice that the page is out of date (empty: none), with a Refresh action.
    pub pill: Signal<String>,
    /// The server's measure job (the header's Measure button).
    pub job: Signal<MeasureJob>,
}

impl Bus {
    pub(super) fn new() -> Bus {
        Bus {
            pill: create_signal(String::new()),
            job: create_signal(MeasureJob::Idle),
        }
    }
}

/// The stamps seen so far on this page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Seen {
    pub cache: Option<u64>,
    pub config: Option<u64>,
}

/// What to do about one event.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Fetch the change and the levels and patch the page in place.
    RefreshChange,
    /// Show this pill.
    Pill(&'static str),
    Job(MeasureJob),
    Nothing,
}

pub const PILL_CACHE: &str = "Code re-measured";
pub const PILL_CONFIG: &str = "Config changed on disk";
pub const PILL_CHANGE: &str = "The design change was opened or closed elsewhere";

/// The action for `ev`, given the open change on screen (`(open, head)`) and what was `seen`.
pub fn on_event(seen: &mut Seen, ev: &ServerEvent, change: (bool, u64)) -> Action {
    let newer = |slot: &mut Option<u64>, v: u64| slot.replace(v).is_some_and(|old| old != v);
    match ev {
        ServerEvent::Change { open, head } => match (change.0, *open) {
            (true, true) if *head != change.1 => Action::RefreshChange,
            (a, b) if a != b => Action::Pill(PILL_CHANGE),
            _ => Action::Nothing,
        },
        ServerEvent::Cache { stamp, .. } if newer(&mut seen.cache, *stamp) => {
            Action::Pill(PILL_CACHE)
        }
        // A config edit inside an open change is the change's own business (`change` events).
        ServerEvent::Config { stamp } if newer(&mut seen.config, *stamp) && !change.0 => {
            Action::Pill(PILL_CONFIG)
        }
        ServerEvent::Measure { job } => Action::Job(job.clone()),
        _ => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_refresh_the_change_or_raise_a_pill() {
        let mut seen = Seen::default();
        let ch = |open, head| ServerEvent::Change { open, head };
        assert_eq!(
            on_event(&mut seen, &ch(true, 3), (true, 3)),
            Action::Nothing
        );
        assert_eq!(
            on_event(&mut seen, &ch(true, 4), (true, 3)),
            Action::RefreshChange
        );
        assert_eq!(
            on_event(&mut seen, &ch(false, 0), (true, 3)),
            Action::Pill(PILL_CHANGE)
        );
        let cache = |stamp| ServerEvent::Cache {
            measured_at: 1,
            stamp,
        };
        assert_eq!(on_event(&mut seen, &cache(5), (false, 0)), Action::Nothing);
        assert_eq!(on_event(&mut seen, &cache(5), (false, 0)), Action::Nothing);
        assert_eq!(
            on_event(&mut seen, &cache(6), (false, 0)),
            Action::Pill(PILL_CACHE)
        );
        let cfg = |stamp| ServerEvent::Config { stamp };
        assert_eq!(on_event(&mut seen, &cfg(1), (false, 0)), Action::Nothing);
        assert_eq!(on_event(&mut seen, &cfg(2), (true, 0)), Action::Nothing);
        assert_eq!(
            on_event(&mut seen, &cfg(3), (false, 0)),
            Action::Pill(PILL_CONFIG)
        );
        let job = ServerEvent::Measure {
            job: MeasureJob::Idle,
        };
        assert_eq!(
            on_event(&mut seen, &job, (false, 0)),
            Action::Job(MeasureJob::Idle)
        );
    }
}
