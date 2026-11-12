//! Browser side effects behind a tiny interface. On native targets (server-side rendering)
//! everything is a no-op, so components can be written once.

use crate::model::{ReviewReply, ReviewRequest};

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
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

    /// Key presses outside text inputs (`in_input` is true inside them); `f` returns whether to
    /// `preventDefault`.
    pub fn on_key(f: impl Fn(String, bool) -> bool + 'static) {
        let cb =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if e.ctrl_key() || e.meta_key() || e.alt_key() {
                    return;
                }
                let tag = e
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|el| el.tag_name())
                    .unwrap_or_default();
                let in_input = matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT");
                if f(e.key(), in_input) {
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

    async fn post(req: ReviewRequest) -> Result<ReviewReply, String> {
        let window = web_sys::window().ok_or("no window")?;
        let init = web_sys::RequestInit::new();
        init.set_method("POST");
        init.set_body(&JsValue::from_str(
            &serde_json::to_string(&req).map_err(|e| e.to_string())?,
        ));
        let headers = web_sys::Headers::new().map_err(|_| "headers")?;
        let _ = headers.set("content-type", "application/json");
        init.set_headers(&headers);
        let resp =
            wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init("/review", &init))
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
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn post_review(
        req: ReviewRequest,
        done: impl FnOnce(Result<ReviewReply, String>) + 'static,
    ) {
        wasm_bindgen_futures::spawn_local(async move { done(post(req).await) });
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
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
    pub fn focus(_: &str) {}
    pub fn reload() {}
    pub fn post_review(_: ReviewRequest, _: impl FnOnce(Result<ReviewReply, String>) + 'static) {}
}

pub use imp::*;
