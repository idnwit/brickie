//! Builds the viewer bundle (wasm + JS glue) into `OUT_DIR/ui/` and sets `cfg(ui_wasm)` so the
//! server embeds it, but only with the `ui` cargo feature. Without the feature, or when the wasm
//! toolchain is missing (with a warning), the server serves the SSR-only, read-only page. The bundle is always built from the
//! current sources in release mode; a prebuilt one is never used.

#[path = "../../../build-support/wasm.rs"]
mod wasm;

use std::path::PathBuf;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(ui_wasm)");
    println!("cargo::rerun-if-changed=build.rs");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../../..").canonicalize().unwrap();
    if !wasm::enabled() {
        println!("cargo::rustc-env=BRICKIE_UI_TAG=none");
        return;
    }
    wasm::rerun_on_sources(&workspace);
    let ui = workspace.join("crates/web/ui");
    match wasm::toolchain(&ui.join("Cargo.toml")) {
        Ok(()) => {
            let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("ui");
            let _ = std::fs::remove_dir_all(&out);
            wasm::build(&workspace, "brickie-ui", &["hydrate"], "brickie_ui", &out);
            println!("cargo::rustc-env=BRICKIE_UI_TAG={}", tag(&out));
            println!("cargo::rustc-cfg=ui_wasm");
        }
        Err(why) => {
            println!("cargo::rustc-env=BRICKIE_UI_TAG=none");
            println!(
                "cargo::warning=viewer bundle not built ({why}): `brickie serve` will serve the read-only SSR page"
            )
        }
    }
}

/// A content hash of the bundle (FNV-1a over the JS glue and the wasm), used in its URLs so a
/// browser never pairs a new page with a cached old bundle.
fn tag(out: &std::path::Path) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for f in ["brickie_ui.js", "brickie_ui_bg.wasm"] {
        for b in std::fs::read(out.join(f)).expect("read the built bundle") {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}
