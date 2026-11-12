//! `GET /api/events`: the Server-Sent Events stream (read-only, no write guard). Each
//! connection first gets the current state (one event of each kind), then every change the
//! watcher sees; a keep-alive comment every 15 s keeps proxies from closing it.

use crate::AppState;
use crate::events::snapshot;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use brickie_web_api::ServerEvent;
use futures_util::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

fn sse(e: &ServerEvent) -> Result<Event, Infallible> {
    let data = serde_json::to_string(e).unwrap_or_default();
    Ok(Event::default().event(e.kind()).data(data))
}

pub async fn events(
    State(st): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = st.events.subscribe();
    let now = match st.events.last() {
        Some(s) => s,
        None => {
            let s = st.clone();
            tokio::task::spawn_blocking(move || snapshot(&s, 0))
                .await
                .expect("snapshot task")
        }
    };
    let first = stream::iter(now.events().iter().map(sse).collect::<Vec<_>>());
    let live = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(e) => return Some((sse(&e), rx)),
                // Missed some: the next event (or a reconnect) carries the current state.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(first.chain(live)).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
