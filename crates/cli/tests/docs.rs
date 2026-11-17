//! Docs against code: `llms.txt` names every config key the loader accepts, every `toml` block in
//! the docs loads, relative links in `docs/` resolve and `llms.txt` links nowhere.
//! (Diagnostic codes in `llms.txt`: `guide_documents_every_diagnostic_code`.)

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn llms() -> String {
    read(&root().join("docs/llms.txt"))
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// Doc files: `docs/**/*.md`, `docs/llms.txt` and `README.md`.
fn docs() -> Vec<PathBuf> {
    let mut out = vec![root().join("docs/llms.txt"), root().join("README.md")];
    files(&root().join("docs"), "md", &mut out);
    out
}

/// Loads `toml` as a `brickie.toml`.
fn load(toml: &str) -> anyhow::Result<()> {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("brickie.toml");
    std::fs::write(&config, toml).unwrap();
    brickie_check::config::load(&config).map(drop)
}

/// The keys a table accepts, read from the loader's unknown-field error.
fn accepted_keys(toml: &str) -> Vec<String> {
    let err = format!("{:#}", load(toml).expect_err("must fail"));
    let list = err
        .split("expected one of ")
        .nth(1)
        .or_else(|| err.split("expected ").nth(1))
        .unwrap_or_else(|| panic!("no field list in: {err}"));
    list.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[test]
fn llms_txt_names_every_config_key() {
    let text = llms();
    let tables = [
        "bogus = 1",
        "[[brick]]\nname = \"a\"\nbogus = 1",
        "[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\nbogus = 1",
        "[[complexity]]\nbogus = 1",
        "[measure]\nbogus = 1",
    ];
    for toml in tables {
        for key in accepted_keys(toml) {
            let found = [format!("{key} ="), format!("{key}]"), format!("`{key}`")]
                .iter()
                .any(|p| text.contains(p.as_str()));
            assert!(found, "llms.txt does not name key `{key}` (table: {toml})");
        }
    }
}

/// Fenced ```toml blocks in `text`.
fn toml_blocks(text: &str) -> Vec<String> {
    text.split("```toml\n")
        .skip(1)
        .filter_map(|rest| rest.split("\n```").next().map(str::to_string))
        .collect()
}

#[test]
fn every_toml_block_in_the_docs_loads() {
    for file in docs() {
        for block in toml_blocks(&read(&file)) {
            if let Err(e) = load(&block) {
                // A fragment may name bricks and tags declared elsewhere, and a composite's own
                // file or an include is not part of the example; syntax, keys and values must
                // still be valid.
                let msg = format!("{e:#}");
                let external = msg.contains(": read ")
                    || msg.starts_with("read ")
                    || msg.contains(": unknown tag '")
                    || msg.contains(": unknown brick '");
                assert!(
                    external,
                    "{}: toml block does not load: {msg}\n{block}",
                    file.display()
                );
            }
        }
    }
}

/// Relative link targets `](target)` in markdown text, without anchors.
fn links(text: &str) -> Vec<String> {
    text.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .filter(|t| !t.contains("://") && !t.starts_with('#') && !t.starts_with("mailto:"))
        .map(|t| t.split('#').next().unwrap_or(t).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

#[test]
fn relative_links_in_the_docs_resolve() {
    for file in docs()
        .into_iter()
        .filter(|f| f.extension().is_some_and(|x| x == "md"))
    {
        let dir = file.parent().unwrap();
        for target in links(&read(&file)) {
            assert!(
                dir.join(&target).exists(),
                "{}: broken link {target}",
                file.display()
            );
        }
    }
}

#[test]
fn llms_txt_links_nowhere() {
    let text = llms();
    assert!(
        links(&text).is_empty(),
        "llms.txt has links: {:?}",
        links(&text)
    );
    assert!(!text.contains("docs/"), "llms.txt points at repo files");
}
