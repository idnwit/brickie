//! With the `ui` feature, builds the code viewer bundle into `OUT_DIR/bundle/` and embeds every
//! file there: the page, the app wasm and JS, and Monaco's `vs/` files (fetched once from the npm
//! registry, hash checked, cached under the target directory; needs curl, tar and openssl).
//! Without the feature, or when a prerequisite is missing (with a warning), nothing is embedded
//! and the router serves a page saying the viewer is not built.

#[path = "../../../../build-support/wasm.rs"]
mod wasm;

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const MONACO: &str = "0.52.2";
const SHA512: &str =
    "GEQWEZmfkOGLdd3XK8ryrfWz3AIP8YymVXiPHEdewrUq7mh0qrKrfHLNCXcbB6sTnMLnOZ3ztSiKcciFUkIJwQ==";
const MONACO_FILES: &[&str] = &[
    "loader.js",
    "editor/editor.main.js",
    "editor/editor.main.css",
    "base/worker/workerMain.js",
    "basic-languages/rust/rust.js",
    "base/browser/ui/codicons/codicon/codicon.ttf",
];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn sh(script: &str) -> Result<(), String> {
    match Command::new("sh").args(["-c", script]).status() {
        Ok(s) if s.success() => Ok(()),
        _ => Err("fetching monaco-editor failed (needs curl, tar, openssl and network)".into()),
    }
}

/// Monaco's needed files under `cache/vs`, fetched when the cached version differs.
fn monaco(cache: &Path) -> Result<(), String> {
    let vs = cache.join("vs");
    if std::fs::read_to_string(vs.join("VERSION")).is_ok_and(|v| v.trim() == MONACO) {
        return Ok(());
    }
    let _ = std::fs::remove_dir_all(cache);
    std::fs::create_dir_all(cache).unwrap();
    let files = MONACO_FILES
        .iter()
        .map(|f| format!("package/min/vs/{f}"))
        .collect::<Vec<_>>()
        .join(" ");
    let c = cache.display();
    sh(&format!(
        "set -e; curl -sfL https://registry.npmjs.org/monaco-editor/-/monaco-editor-{MONACO}.tgz -o '{c}/m.tgz'; \
         got=$(openssl dgst -sha512 -binary '{c}/m.tgz' | openssl base64 -A); \
         [ \"$got\" = '{SHA512}' ] || {{ echo 'monaco integrity mismatch' >&2; exit 1; }}; \
         tar -xzf '{c}/m.tgz' -C '{c}' {files}; rm '{c}/m.tgz'; mkdir -p '{c}/vs'; \
         cp -r '{c}/package/min/vs/.' '{c}/vs/'; rm -rf '{c}/package'"
    ))?;
    std::fs::write(vs.join("VERSION"), MONACO).unwrap();
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) {
    let mut files = Vec::new();
    walk(from, &mut files);
    for f in files {
        let dest = to.join(f.strip_prefix(from).unwrap());
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(&f, dest).unwrap();
    }
}

fn build(workspace: &Path, bundle: &Path) -> Result<(), String> {
    wasm::toolchain(&workspace.join("crates/lib/codeview/app/Cargo.toml"))?;
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let cache = out
        .ancestors()
        .nth(4)
        .unwrap_or(&out)
        .join("brickie-monaco");
    monaco(&cache)?;
    wasm::build(workspace, "codeview-app", &[], "codeview_app", bundle);
    copy_tree(&cache.join("vs"), &bundle.join("vs"));
    let stat = workspace.join("crates/lib/codeview/app/static");
    copy_tree(&stat, bundle);
    Ok(())
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../../../..").canonicalize().unwrap();
    let bundle = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundle");
    let _ = std::fs::remove_dir_all(&bundle);
    std::fs::create_dir_all(&bundle).unwrap();
    if wasm::enabled() {
        wasm::rerun_on_sources(&workspace);
        if let Err(why) = build(&workspace, &bundle) {
            let _ = std::fs::remove_dir_all(&bundle);
            std::fs::create_dir_all(&bundle).unwrap();
            println!("cargo::warning=code viewer bundle not built ({why}): /code/ will say so");
        }
    }
    let mut files = Vec::new();
    walk(&bundle, &mut files);
    files.sort();
    let mut src = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let rel = f
            .strip_prefix(&bundle)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        writeln!(src, "    ({rel:?}, include_bytes!({:?})),", f.display()).unwrap();
    }
    src.push_str("];\n");
    std::fs::write(bundle.parent().unwrap().join("bundle.rs"), src).unwrap();
}
