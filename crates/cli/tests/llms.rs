//! `brickie llms.txt` prints the static guide and works anywhere, without a config or cache.

use std::process::Command;

#[test]
fn llms_txt_prints_the_guide_without_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_brickie"))
        .arg("llms.txt")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("# brickie"));
    assert!(text.contains("brickie measure") && text.contains("deps/cycle"));
}
