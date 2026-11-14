//! Browser side effects behind a tiny interface. On native targets (server-side rendering)
//! everything is a no-op, so components can be written once.
#[cfg(target_arch = "wasm32")]
mod imp {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    pub fn push_url(url: &str) {
        if let Some(h) = web_sys::window().and_then(|w| w.history().ok()) {
            let _ = h.push_state_with_url(&JsValue::NULL, "", Some(url));
        }
    }

    pub fn search() -> String {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default()
    }

    pub fn on_popstate(f: impl Fn() + 'static) {
        let cb = Closure::<dyn Fn()>::new(f);
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("popstate", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    /// Key presses (Ctrl/Cmd chords as `Ctrl+<key>` / `Ctrl+Shift+<key>`) outside text inputs (`in_input` is true inside them); `f` returns whether to
    /// `preventDefault`.
    pub fn on_key(f: impl Fn(String, bool) -> bool + 'static) {
        let cb =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if e.alt_key() {
                    return;
                }
                // Ctrl/Cmd chords arrive as `Ctrl+<key>` (lower case).
                let key = if (e.ctrl_key() || e.meta_key()) && e.shift_key() {
                    format!("Ctrl+Shift+{}", e.key().to_lowercase())
                } else if e.ctrl_key() || e.meta_key() {
                    format!("Ctrl+{}", e.key().to_lowercase())
                } else {
                    e.key()
                };
                let tag = e
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|el| el.tag_name())
                    .unwrap_or_default();
                let in_input = matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT");
                if f(key, in_input) {
                    e.prevent_default();
                }
            });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    /// `(left, top, width, height)` of `#stage`.
    pub fn stage_rect() -> Option<(f64, f64, f64, f64)> {
        let r = web_sys::window()?
            .document()?
            .get_element_by_id("stage")?
            .get_bounding_client_rect();
        (r.width() > 0.0).then(|| (r.left(), r.top(), r.width(), r.height()))
    }

    /// Window-level mouse events (`mousemove` / `mouseup`): `f(client x, client y)`.
    pub fn on_mouse(kind: &'static str, f: impl Fn(f64, f64) + 'static) {
        let cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            f(e.client_x() as f64, e.client_y() as f64)
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    pub fn on_resize(f: impl Fn() + 'static) {
        let cb = Closure::<dyn Fn()>::new(f);
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    /// Run `f` once before the next paint (`requestAnimationFrame`), when the DOM is in place.
    pub fn next_frame(f: impl FnOnce() + 'static) {
        let cb = Closure::once_into_js(f);
        if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(cb.unchecked_ref());
        }
    }

    /// Copy `text` to the clipboard (needs a secure context; fails silently otherwise).
    pub fn copy_text(text: &str) {
        if let Some(w) = web_sys::window() {
            let _ = w.navigator().clipboard().write_text(text);
        }
    }

    /// Run `f` once after `ms` milliseconds (`setTimeout`).
    pub fn after(ms: i32, f: impl FnOnce() + 'static) {
        let cb = Closure::once_into_js(f);
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
        }
    }

    /// The `href` of the breadcrumb link under client point `(x, y)`.
    pub fn crumb_at(x: f64, y: f64) -> Option<String> {
        let doc = web_sys::window()?.document()?;
        let el = doc.element_from_point(x as f32, y as f32)?;
        el.closest("a.crumb").ok()??.get_attribute("href")
    }

    /// Whether a text field (input, textarea, select) has the focus.
    pub fn typing() -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .is_some_and(|e| matches!(e.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
    }

    /// Blur the focused element.
    pub fn blur_active() {
        if let Some(e) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = e.blur();
        }
    }

    pub fn reload() {
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    }

    pub fn focus(id: &str) {
        if let Some(e) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = e.focus();
        }
    }

    /// Go to `url` (a full page load).
    pub fn goto(url: &str) {
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_href(url);
        }
    }

    async fn fetch(path: &str, body: Option<String>) -> Result<String, String> {
        let window = web_sys::window().ok_or("no window")?;
        let init = web_sys::RequestInit::new();
        init.set_method(if body.is_some() { "POST" } else { "GET" });
        if let Some(body) = body {
            init.set_body(&JsValue::from_str(&body));
        }
        let headers = web_sys::Headers::new().map_err(|_| "headers")?;
        let _ = headers.set("content-type", "application/json");
        init.set_headers(&headers);
        let resp =
            wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(path, &init))
                .await
                .map_err(|_| "request failed".to_string())?;
        let resp: web_sys::Response = resp.dyn_into().map_err(|_| "bad response")?;
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|_| "no body")?)
            .await
            .map_err(|_| "no body")?
            .as_string()
            .unwrap_or_default();
        if !resp.ok() {
            return Err(format!("{}: {text}", resp.status()));
        }
        Ok(text)
    }

    /// `message` events of the window (iframes posting to this page), as JSON text.
    pub fn on_message(f: impl Fn(String) + 'static) {
        let cb = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if let Some(t) = e.data().as_string() {
                f(t);
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    /// Post the JSON text `msg` to the window of the iframe `#id`.
    pub fn post_to_frame(id: &str, msg: &str) {
        let win = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
            .and_then(|e| e.dyn_into::<web_sys::HtmlIFrameElement>().ok())
            .and_then(|f| f.content_window());
        if let Some(w) = win {
            let _ = w.post_message(&JsValue::from_str(msg), "*");
        }
    }

    /// POST the JSON text `body` to `path`; `done` gets the response text or an error message.
    pub fn post_json(
        path: &'static str,
        body: String,
        done: impl FnOnce(Result<String, String>) + 'static,
    ) {
        wasm_bindgen_futures::spawn_local(async move { done(fetch(path, Some(body)).await) });
    }

    /// GET `path`; `done` gets the response text or an error message.
    pub fn get_json(path: &'static str, done: impl FnOnce(Result<String, String>) + 'static) {
        wasm_bindgen_futures::spawn_local(async move { done(fetch(path, None).await) });
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn push_url(_: &str) {}
    pub fn search() -> String {
        String::new()
    }
    pub fn on_popstate(_: impl Fn() + 'static) {}
    pub fn on_key(_: impl Fn(String, bool) -> bool + 'static) {}
    pub fn stage_rect() -> Option<(f64, f64, f64, f64)> {
        None
    }
    pub fn on_mouse(_: &'static str, _: impl Fn(f64, f64) + 'static) {}
    pub fn on_resize(_: impl Fn() + 'static) {}
    pub fn next_frame(_: impl FnOnce() + 'static) {}
    pub fn focus(_: &str) {}
    pub fn reload() {}
    pub fn blur_active() {}
    pub fn typing() -> bool {
        false
    }
    pub fn crumb_at(_: f64, _: f64) -> Option<String> {
        None
    }
    pub fn after(_: i32, _: impl FnOnce() + 'static) {}
    pub fn on_message(_: impl Fn(String) + 'static) {}
    pub fn post_to_frame(_: &str, _: &str) {}
    pub fn post_json(_: &'static str, _: String, _: impl FnOnce(Result<String, String>) + 'static) {
    }
    pub fn get_json(_: &'static str, _: impl FnOnce(Result<String, String>) + 'static) {}
    pub fn goto(_: &str) {}
    pub fn copy_text(_: &str) {}
}

mod live;

pub use imp::*;
pub use live::*;
