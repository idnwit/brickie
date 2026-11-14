//! What the page does with a Server-Sent Event (`/api/events`): refresh the design in place, follow the measure job, or show a "refresh" pill when the page data (cache, config)
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
    /// Fetch the design and the levels and patch the page in place.
    RefreshDesign,
    /// Show this pill.
    Pill(&'static str),
    Job(MeasureJob),
    Nothing,
}

pub const PILL_CACHE: &str = "Code re-measured";
pub const PILL_CONFIG: &str = "Config changed on disk";

/// The action for `ev`, given the design version on screen (`None`: not in edit mode) and
/// what was `seen`.
pub fn on_event(seen: &mut Seen, ev: &ServerEvent, on_screen: Option<u64>) -> Action {
    let newer = |slot: &mut Option<u64>, v: u64| slot.replace(v).is_some_and(|old| old != v);
    match ev {
        ServerEvent::Design { v, .. } => match on_screen {
            Some(now) if now != *v => Action::RefreshDesign,
            _ => Action::Nothing,
        },
        ServerEvent::Cache { stamp, .. } if newer(&mut seen.cache, *stamp) => {
            Action::Pill(PILL_CACHE)
        }
        // In edit mode a config edit is a design version (`design` events).
        ServerEvent::Config { stamp } if newer(&mut seen.config, *stamp) && on_screen.is_none() => {
            Action::Pill(PILL_CONFIG)
        }
        ServerEvent::Measure { job } => Action::Job(job.clone()),
        _ => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_web_api::design::By;

    #[test]
    fn events_refresh_the_design_or_raise_a_pill() {
        let mut seen = Seen::default();
        let d = |v| ServerEvent::Design {
            hash: "h".into(),
            v,
            by: By::Agent,
        };
        assert_eq!(on_event(&mut seen, &d(3), Some(3)), Action::Nothing);
        assert_eq!(on_event(&mut seen, &d(4), Some(3)), Action::RefreshDesign);
        assert_eq!(on_event(&mut seen, &d(4), None), Action::Nothing);
        let cache = |stamp| ServerEvent::Cache {
            measured_at: 1,
            stamp,
        };
        assert_eq!(on_event(&mut seen, &cache(5), None), Action::Nothing);
        assert_eq!(on_event(&mut seen, &cache(5), None), Action::Nothing);
        assert_eq!(
            on_event(&mut seen, &cache(6), None),
            Action::Pill(PILL_CACHE)
        );
        let cfg = |stamp| ServerEvent::Config { stamp };
        assert_eq!(on_event(&mut seen, &cfg(1), None), Action::Nothing);
        assert_eq!(on_event(&mut seen, &cfg(2), Some(0)), Action::Nothing);
        assert_eq!(
            on_event(&mut seen, &cfg(3), None),
            Action::Pill(PILL_CONFIG)
        );
        let job = ServerEvent::Measure {
            job: MeasureJob::Idle,
        };
        assert_eq!(
            on_event(&mut seen, &job, None),
            Action::Job(MeasureJob::Idle)
        );
    }
}
