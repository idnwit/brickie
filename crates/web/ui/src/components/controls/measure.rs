//! The header's Measure button and cache freshness ("measured 4m ago", "stale: N files"). The
//! button starts the server's measure job; while it runs the header polls its state and shows
//! the step, and when it is done the page reloads with the new cache.

use crate::components::call::{call, get};
use crate::platform;
use crate::state::Click;
use brickie_web_api::*;
use sycamore::prelude::*;

/// Fetch the job state; keep polling while it runs, reload the page once it is done.
fn poll(job: Signal<MeasureJob>, watching: bool) {
    get::<MeasureStatus>(move |r| match r {
        Ok(j) => {
            let (running, done) = (j.running(), matches!(j, MeasureJob::Done { .. }));
            job.set(j);
            if running {
                platform::after(1000, move || poll(job, true));
            } else if done && watching {
                platform::reload();
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

pub fn measure(cache: &CacheInfo) -> View {
    let job = create_signal(MeasureJob::Idle);
    // A job started elsewhere (another tab, the publish dialog) is picked up on load.
    on_mount(move || poll(job, false));
    let (fresh, class) = freshness(cache);
    let tip = format!("Cache written {}", cache.measured);
    let start = move |_: Click| {
        job.set(MeasureJob::Running {
            step: "starting".into(),
            started: 0,
        });
        start_measure(move |r| match r {
            Ok(_) => poll(job, true),
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
        button(id="measure", title="Run brickie measure in the server, then reload",
            disabled=move || job.with(|j| j.running()), on:click=start) { (label) }
        span(id="freshness") { (note) }
    }
}
