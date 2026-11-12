//! Embeds the viewer bundle built by `scripts/build-codeview.sh` (`assets/codeview/`: the page,
//! the app wasm and JS, Monaco's `vs/` files). Every file found there is embedded; when the
//! directory is missing the router serves a page saying how to build it, never a build error.

use std::fmt::Write;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.file_name().is_some_and(|n| n != ".gitkeep") {
            out.push(p);
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = manifest.join("../../../../assets/codeview");
    println!("cargo::rerun-if-changed={}", dir.display());
    println!("cargo::rerun-if-changed=build.rs");
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    let mut src = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let rel = f
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let abs = f.canonicalize().unwrap();
        writeln!(src, "    ({rel:?}, include_bytes!({:?})),", abs.display()).unwrap();
    }
    src.push_str("];\n");
    if !files.iter().any(|f| f.ends_with("codeview_app_bg.wasm")) {
        println!(
            "cargo::warning=assets/codeview/ has no viewer bundle: /code/ will say so (run scripts/build-codeview.sh)"
        );
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundle.rs");
    std::fs::write(out, src).unwrap();
}
