//! Browser entry: hydrate the server-rendered app from the embedded JSON.

use crate::{APP_ID, DATA_ID, components, platform};
use brickie_web_api::{PageData, Params};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static ROOT: RefCell<Option<sycamore::reactive::RootHandle>> = const { RefCell::new(None) };
}

/// Hydrate `#app` using the JSON in `#brickie-data` and the current URL's params. Calling it
/// again disposes the previous root first.
#[wasm_bindgen]
pub fn hydrate() -> Result<(), JsValue> {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let host = doc.get_element_by_id(APP_ID).ok_or("no #app element")?;
    let json = doc
        .get_element_by_id(DATA_ID)
        .and_then(|e| e.text_content())
        .ok_or("no embedded data")?;
    let data: PageData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let params = Params::from_query(&platform::search());
    ROOT.with(|r| {
        if let Some(old) = r.borrow_mut().take() {
            old.dispose();
        }
        let root = sycamore::reactive::create_root(|| {
            sycamore::web::hydrate_in_scope(|| components::app(data, params), &host);
        });
        *r.borrow_mut() = Some(root);
    });
    Ok(())
}
