//! The measure job: `GET`/`POST /api/measure`, the write guard and single flight, with a
//! stand-in runner (no rust-analyzer).

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A router whose runner counts its runs and blocks until `release` is set; fails on `fail`.
struct Job {
    app: axum::Router,
    runs: Arc<AtomicUsize>,
    release: Arc<AtomicBool>,
}

fn job(s: &Site, fail: bool) -> Job {
    let (runs, release) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let (r, g) = (runs.clone(), release.clone());
    let runner: crate::measure::Runner =
        Arc::new(move |_: &std::path::Path, progress: &dyn Fn(&str)| {
            r.fetch_add(1, Ordering::SeqCst);
            progress("rust-analyzer scip");
            while !g.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            match fail {
                true => Err(anyhow::anyhow!("rust-analyzer scip failed")),
                false => Ok(()),
            }
        });
    let app = router(AppState::new(s.config()).with_runner(runner));
    Job { app, runs, release }
}

async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (st, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn post(origin: &str) -> Request<Body> {
    Request::post("/api/measure")
        .header("host", "127.0.0.1:3000")
        .header("origin", origin)
        .body(Body::empty())
        .unwrap()
}

async fn status(app: &axum::Router) -> Value {
    send(
        app,
        Request::get("/api/measure").body(Body::empty()).unwrap(),
    )
    .await
    .1
}

/// Polls until the job leaves `running`.
async fn settled(app: &axum::Router) -> Value {
    for _ in 0..400 {
        let v = status(app).await;
        if v["state"] != "running" {
            return v;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the measure job did not finish");
}

#[tokio::test(flavor = "multi_thread")]
async fn measure_is_guarded_and_single_flight() {
    let s = Site::new();
    let j = job(&s, false);
    assert_eq!(status(&j.app).await, json!({"state": "idle"}));

    let (st, _) = send(&j.app, post("http://evil.test")).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(status(&j.app).await["state"], "idle");

    let (st, first) = send(&j.app, post("http://127.0.0.1:3000")).await;
    assert_eq!(
        (st, first["state"].clone()),
        (StatusCode::OK, json!("running"))
    );
    let (_, again) = send(&j.app, post("http://127.0.0.1:3000")).await;
    assert_eq!(
        again["started"], first["started"],
        "the running job is returned"
    );
    j.release.store(true, Ordering::SeqCst);
    assert_eq!(settled(&j.app).await["state"], "done");
    assert_eq!(j.runs.load(Ordering::SeqCst), 1, "one run for two starts");

    // After it finished, a new start runs again.
    send(&j.app, post("http://127.0.0.1:3000")).await;
    assert_eq!(settled(&j.app).await["state"], "done");
    assert_eq!(j.runs.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_measure_reports_error_and_log_tail() {
    let s = Site::new();
    let j = job(&s, true);
    j.release.store(true, Ordering::SeqCst);
    send(&j.app, post("http://127.0.0.1:3000")).await;
    let v = settled(&j.app).await;
    assert_eq!(v["state"], "failed");
    assert_eq!(v["error"], "rust-analyzer scip failed");
    assert_eq!(
        v["log"],
        json!(["rust-analyzer scip", "error: rust-analyzer scip failed"])
    );
}

#[tokio::test]
async fn page_reports_cache_age_and_stale_files() {
    let s = Site::new();
    let page = s.get("/").await.1;
    assert!(page.contains("\"age_secs\":"), "cache age in the page data");
    // link.rs is cached with another file's hash: one stale file.
    assert!(page.contains("\"stale_files\":1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn jobs_run_the_real_measure_and_release_the_lock() {
    use crate::measure::{Jobs, measure_runner};
    // The site is no cargo workspace: the real runner fails at `cargo metadata`.
    let s = Site::new();
    let jobs = Arc::new(Jobs::new(measure_runner()));
    assert!(jobs.start(s.config()).running());
    let mut end = jobs.status();
    for _ in 0..2000 {
        if !end.running() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        end = jobs.status();
    }
    assert!(
        matches!(end, brickie_web_api::MeasureJob::Failed { .. }),
        "{end:?}"
    );
    let lock = s.config().parent().unwrap().join(".brickie/cache.lock");
    assert!(
        !lock.exists(),
        "the cache lock is released after a failed run"
    );
}
