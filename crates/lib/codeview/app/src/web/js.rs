//! The browser behind small functions: the Monaco glue (`static/glue.js`, namespace `cv`), the
//! endpoints, the host page's messages, the URL and the colour scheme.

use codeview_port::{Endpoint, ToHost, ToViewer};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_namespace = cv)]
extern "C" {
    /// Create the editor in `el` once Monaco has loaded; the callbacks get `(line, byte col)`.
    pub fn init(
        el: &web_sys::Element,
        dark: bool,
        on_def: &js_sys::Function,
        on_refs: &js_sys::Function,
        on_quick: &js_sys::Function,
    );
    /// Whether a model for `path` is loaded.
    pub fn has(path: &str) -> bool;
    /// Load `text` as the model of `path`.
    pub fn load(path: &str, text: &str, lang: &str);
    /// Show `path` (loaded), revealing `line` (0: keep the position) and byte column `col`.
    pub fn show(path: &str, line: u32, col: u32);
    /// Drop the model of `path`.
    pub fn dispose(path: &str);
    /// Replace the marks of `path` (JSON array of port `Mark`s).
    #[wasm_bindgen(js_name = setMarks)]
    pub fn set_marks(path: &str, json: &str);
    /// Replace the bands of `path` (JSON array of port `Band`s).
    #[wasm_bindgen(js_name = setBands)]
    pub fn set_bands(path: &str, json: &str);
    /// Show or hide the bands gutter.
    #[wasm_bindgen(js_name = showBands)]
    pub fn show_bands(on: bool);
    pub fn theme(dark: bool);
}

fn window() -> web_sys::Window {
    web_sys::window().expect("window")
}

pub fn element(id: &str) -> Option<web_sys::Element> {
    window().document()?.get_element_by_id(id)
}

pub fn focus(id: &str) {
    if let Some(e) = element(id).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = e.focus();
    }
}

pub fn search() -> String {
    window().location().search().unwrap_or_default()
}

pub fn prefers_dark() -> bool {
    window()
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

pub fn set_dark_class(dark: bool) {
    if let Some(root) = window().document().and_then(|d| d.document_element()) {
        let _ = root.set_attribute("class", if dark { "dark" } else { "light" });
    }
}

/// Whether the viewer runs inside a host page's iframe.
fn framed() -> bool {
    window()
        .parent()
        .ok()
        .flatten()
        .is_some_and(|p| p != window())
}

/// Post `msg` to the host page (no-op when standalone).
pub fn to_host(msg: &ToHost) {
    if !framed() {
        return;
    }
    if let (Some(p), Ok(json)) = (window().parent().ok().flatten(), serde_json::to_string(msg)) {
        let _ = p.post_message(&JsValue::from_str(&json), "*");
    }
}

/// Follow a mark's link: the host page handles it, or this tab goes there when standalone.
pub fn follow(href: &str) {
    if framed() {
        to_host(&ToHost::Link {
            href: href.to_string(),
        });
    } else {
        let _ = window().location().set_href(href);
    }
}

/// Messages from the host page (JSON text or objects that parse as [`ToViewer`]).
pub fn on_message(f: impl Fn(ToViewer) + 'static) {
    let cb = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
        let data = e.data();
        let text = data.as_string().or_else(|| {
            js_sys::JSON::stringify(&data)
                .ok()
                .and_then(|s| s.as_string())
        });
        if let Some(m) = text.and_then(|t| serde_json::from_str(&t).ok()) {
            f(m);
        }
    });
    let _ = window().add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Window key presses with Ctrl or Cmd: `f(key)` returns whether it handled them.
pub fn on_ctrl_key(f: impl Fn(&str) -> bool + 'static) {
    let cb = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        if (e.ctrl_key() || e.meta_key()) && f(&e.key()) {
            e.prevent_default();
        }
    });
    let _ = window().add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// A JS function `(line, col)` calling `f`.
pub fn callback2(f: impl Fn(u32, u32) + 'static) -> js_sys::Function {
    Closure::<dyn Fn(u32, u32)>::new(f)
        .into_js_value()
        .unchecked_into()
}

pub fn callback0(f: impl Fn() + 'static) -> js_sys::Function {
    Closure::<dyn Fn()>::new(f).into_js_value().unchecked_into()
}

async fn post_text(path: &str, body: String) -> Result<String, String> {
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&JsValue::from_str(&body));
    let headers = web_sys::Headers::new().map_err(|_| "headers")?;
    let _ = headers.set("content-type", "application/json");
    init.set_headers(&headers);
    let resp = wasm_bindgen_futures::JsFuture::from(window().fetch_with_str_and_init(path, &init))
        .await
        .map_err(|_| "network error")?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "bad response")?;
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|_| "no body")?)
        .await
        .map_err(|_| "no body")?
        .as_string()
        .unwrap_or_default();
    if !resp.ok() {
        return Err(format!("{} {}", resp.status(), text));
    }
    Ok(text)
}

/// Call endpoint `E` with `req`; `done` gets the decoded answer or an error message.
pub fn call<E: Endpoint + 'static>(
    req: &E::Req,
    done: impl FnOnce(Result<E::Res, String>) + 'static,
) {
    let body = serde_json::to_string(req).unwrap_or_default();
    wasm_bindgen_futures::spawn_local(async move {
        let res = post_text(E::PATH, body)
            .await
            .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()));
        done(res);
    });
}
