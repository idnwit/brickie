//! The browser's history (Back / Forward) and the server's event stream (`EventSource`).

#[cfg(target_arch = "wasm32")]
mod imp {
    use crate::platform::typing;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    /// Back (`-1`) or forward (`1`) in the browser's history.
    pub fn history_go(delta: i32) {
        if let Some(h) = web_sys::window().and_then(|w| w.history().ok()) {
            let _ = h.go_with_delta(delta);
        }
    }

    /// Back / forward gestures: `Alt+Left` / `Alt+Right` (not while typing) and mouse buttons
    /// 4 / 5; `f(-1 | 1)`. The browser's own handling is prevented, so a step happens once.
    pub fn on_history_nav(f: impl Fn(i32) + 'static) {
        let f = std::rc::Rc::new(f);
        let g = f.clone();
        let key =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                let d = match e.key().as_str() {
                    "ArrowLeft" => -1,
                    "ArrowRight" => 1,
                    _ => return,
                };
                if !e.alt_key() || e.ctrl_key() || e.meta_key() || e.shift_key() {
                    return;
                }
                e.prevent_default();
                if !typing() {
                    g(d);
                }
            });
        let mouse = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let d = match e.button() {
                3 => -1,
                4 => 1,
                _ => return,
            };
            e.prevent_default();
            f(d);
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
            let _ = w.add_event_listener_with_callback("mouseup", mouse.as_ref().unchecked_ref());
        }
        key.forget();
        mouse.forget();
    }

    thread_local! {
        static LIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// Listen to the Server-Sent Events at `url`: `f(data)` for each event of `kinds`. The
    /// browser reconnects by itself. False when `EventSource` is not available.
    pub fn on_events(url: &str, kinds: &[&'static str], f: impl Fn(String) + 'static) -> bool {
        let Ok(es) = web_sys::EventSource::new(url) else {
            return false;
        };
        let f = std::rc::Rc::new(f);
        for k in kinds {
            let g = f.clone();
            let cb =
                Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
                    if let Some(s) = e.data().as_string() {
                        g(s);
                    }
                });
            let _ = es.add_event_listener_with_callback(k, cb.as_ref().unchecked_ref());
            cb.forget();
        }
        std::mem::forget(es);
        LIVE.with(|l| l.set(true));
        true
    }

    /// The event stream is on (else the page polls).
    pub fn events_live() -> bool {
        LIVE.with(|l| l.get())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn history_go(_: i32) {}
    pub fn on_events(_: &str, _: &[&'static str], _: impl Fn(String) + 'static) -> bool {
        false
    }
    pub fn events_live() -> bool {
        false
    }
    pub fn on_history_nav(_: impl Fn(i32) + 'static) {}
}

pub use imp::*;
