//! The header's Measure button and cache freshness ("measured 4m ago", "stale: N files"). The
//! button starts the server's measure job; its progress arrives as `measure` events (or, without
//! the event stream, by polling), and a finished measure raises the "refresh" pill instead of
//! reloading the page under the user.

use crate::components::call::{call, get};
use crate::platform;
use crate::state::Click;
use brickie_web_api::*;
use sycamore::prelude::*;

/// Without the event stream: fetch the job state, and again while it runs.
fn poll(job: Signal<MeasureJob>) {
    get::<MeasureStatus>(move |r| match r {
        Ok(j) => {
            let running = j.running();
            job.set(j);
            if running {
                platform::after(1000, move || poll(job));
            }
        }
        Err(e) => job.set(failed(e)),
    });
}

fn failed(error: String) -> MeasureJob {
    MeasureJob::Failed {
        error,
        log: Vec::new(),
    }
}

/// Start the measure job; `then` gets the job state (or the error) once the server answered.
pub fn start_measure(then: impl FnOnce(Result<MeasureJob, String>) + 'static) {
    call::<MeasureStart>(&(), then);
}

fn freshness(cache: &CacheInfo) -> (String, &'static str) {
    let age = cache.age_secs.map_or_else(|| "unknown".into(), age);
    match cache.stale_files {
        0 => (format!("measured {age}"), "fresh"),
        n => (
            format!(
                "stale: {n} file{} \u{00B7} measured {age}",
                if n == 1 { "" } else { "s" }
            ),
            "stale",
        ),
    }
}

/// The Measure button and the cache's freshness; `job` follows the server's measure job.
pub fn measure(cache: &CacheInfo, job: Signal<MeasureJob>) -> View {
    let (fresh, class) = freshness(cache);
    let tip = format!("Cache written {}", cache.measured);
    let start = move |_: Click| {
        job.set(MeasureJob::Running {
            step: "starting".into(),
            started: 0,
        });
        start_measure(move |r| match r {
            Ok(j) if platform::events_live() => job.set(j),
            Ok(_) => poll(job),
            Err(e) => job.set(failed(e)),
        });
    };
    let label = move || match job.get_clone() {
        MeasureJob::Running { step, .. } => format!("Measuring: {step}\u{2026}"),
        _ => "Measure".to_string(),
    };
    let note = move || match job.get_clone() {
        MeasureJob::Failed { error, log } => {
            let tip = log.join("\n");
            view! { span(class="err-text", title=tip) { "measure failed: " (error) } }
        }
        _ => {
            let (tip, fresh) = (tip.clone(), fresh.clone());
            view! { span(class=format!("age {class}"), title=tip) { (fresh) } }
        }
    };
    view! {
        button(id="measure", title="Run brickie measure in the server",
            disabled=move || job.with(|j| j.running()), on:click=start) { (label) }
        span(id="freshness") { (note) }
    }
}

/// "Code re-measured · Refresh": the page data is out of date (an event said so).
pub fn pill(text: Signal<String>) -> View {
    let off = move || text.with(String::is_empty);
    view! {
        span(id="pill", class="pill", style=move || if off() { "display:none" } else { "" }) {
            (move || text.get_clone())
            button(on:click=move |_: Click| platform::reload()) { "Refresh" }
            button(class="x", title="Dismiss", on:click=move |_: Click| text.set(String::new())) { "\u{2715}" }
        }
    }
}
