//! Shared by the build scripts that embed a wasm bundle (`crates/web/server/build.rs`,
//! `crates/lib/codeview/host/build.rs`), included with `#[path]`.
//!
//! A bundle is built by a nested `cargo build --release --target wasm32-unknown-unknown` into its
//! own target directory (the outer build holds the lock on the main one), then `wasm-bindgen
//! --target web` writes the JS glue and wasm into a directory under `OUT_DIR`. Nothing is read
//! from or written to the source tree.
//!
//! Bundles are built only with the crate's `ui` cargo feature (`enabled()`). The toolchain is
//! optional: when the wasm32 target is missing or `wasm-bindgen` is not the version pinned by the
//! crate, `toolchain()` returns the reason and the caller falls back to its bundle-less build. A compile error in the bundle crate fails the build.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned `wasm-bindgen` version of the manifest (`wasm-bindgen = "=X"`).
fn pinned_version(manifest: &Path) -> String {
    let text = std::fs::read_to_string(manifest).unwrap_or_default();
    text.lines()
        .find_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("wasm-bindgen = \"=")?;
            Some(rest.split('"').next()?.to_string())
        })
        .unwrap_or_default()
}

/// Whether the calling crate was built with its `ui` feature.
pub fn enabled() -> bool {
    std::env::var_os("CARGO_FEATURE_UI").is_some()
}

/// Checks that bundles can be built; `Err` holds a one-line reason with the fix.
pub fn toolchain(pin_manifest: &Path) -> Result<(), String> {
    let want = pinned_version(pin_manifest);
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let sysroot = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if !Path::new(&sysroot)
        .join("lib/rustlib/wasm32-unknown-unknown")
        .is_dir()
    {
        return Err(
            "the wasm32 target is missing: run `rustup target add wasm32-unknown-unknown`".into(),
        );
    }
    let got = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if got.split_whitespace().nth(1) != Some(want.as_str()) {
        return Err(format!(
            "wasm-bindgen {want} is missing (found {:?}): run `cargo install wasm-bindgen-cli --version {want} --locked`",
            got
        ));
    }
    Ok(())
}

/// Builds `package` for wasm and writes `<name>.js` and `<name>_bg.wasm` into `out`.
pub fn build(workspace: &Path, package: &str, features: &[&str], name: &str, out: &Path) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // `<target>/<profile>/build/<pkg>-<hash>/out`: the main target directory is four levels up
    // (five with an explicit `--target`, which is still a directory we may create under).
    let target = out_dir
        .ancestors()
        .nth(4)
        .unwrap_or(&out_dir)
        .join("brickie-wasm");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let home = std::env::var("HOME").unwrap_or_default();
    let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();
    if !home.is_empty() {
        flags.push_str(&format!(" --remap-path-prefix={home}=~"));
    }
    flags.push_str(&format!(" --remap-path-prefix={}=.", workspace.display()));
    let mut cmd = Command::new(cargo);
    cmd.current_dir(workspace)
        .args(["build", "--release", "--locked", "-p", package])
        .args(["--target", "wasm32-unknown-unknown"])
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTFLAGS", flags)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_TARGET")
        .env_remove("CARGO_BUILD_RUSTFLAGS");
    if !features.is_empty() {
        cmd.args(["--features", &features.join(",")]);
    }
    let status = cmd.status().expect("run cargo");
    assert!(status.success(), "building {package} for wasm failed");
    std::fs::create_dir_all(out).unwrap();
    let wasm = target
        .join("wasm32-unknown-unknown/release")
        .join(format!("{}.wasm", package.replace('-', "_")));
    let status = Command::new("wasm-bindgen")
        .args(["--target", "web", "--no-typescript", "--out-dir"])
        .arg(out)
        .args(["--out-name", name])
        .arg(wasm)
        .status()
        .expect("run wasm-bindgen");
    assert!(status.success(), "wasm-bindgen failed for {package}");
}

/// Re-run the calling build script when anything under the workspace crates, the lockfile or the
/// shared helper changes (the bundle crates depend on several workspace crates).
pub fn rerun_on_sources(workspace: &Path) {
    println!(
        "cargo::rerun-if-changed={}",
        workspace.join("crates").display()
    );
    println!(
        "cargo::rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );
    println!(
        "cargo::rerun-if-changed={}",
        workspace.join("Cargo.toml").display()
    );
    println!(
        "cargo::rerun-if-changed={}",
        workspace.join("build-support").display()
    );
}
