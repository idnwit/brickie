//! Detects the browser bundle built by `scripts/build-ui.sh`. When `assets/ui/` holds both
//! files the server embeds them (`cfg(ui_wasm)`); otherwise it serves the SSR-only, read-only
//! page and says so here instead of failing the build.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(ui_wasm)");
    println!("cargo::rerun-if-changed=assets/ui");
    println!("cargo::rerun-if-changed=build.rs");
    let have = ["brickie_ui.js", "brickie_ui_bg.wasm"]
        .iter()
        .all(|f| std::path::Path::new("assets/ui").join(f).is_file());
    if have {
        println!("cargo::rustc-cfg=ui_wasm");
    } else {
        println!(
            "cargo::warning=assets/ui/ has no wasm bundle: `brickie view` will serve the read-only SSR page (run scripts/build-ui.sh to enable the interactive UI)"
        );
    }
}
