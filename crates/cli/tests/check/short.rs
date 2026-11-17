//! Check tests: `--format short` and `--measure`.

use super::support::*;

const RULES: &str = r#"
[[rule]]
from = "*"
to = "port"
reason = "port is open"
[[rule]]
from = "app"
to = "*"
reason = "app may use anything"
[[rule]]
from = "engine"
to = "adapter"
reason = "engine drives adapters"
[[link]]
from = "engine"
to = "adapter"
reason = "needed"
[[link]]
from = "engine"
to = "port"
reason = "needed"
[[link]]
from = "adapter"
to = "port"
reason = "needed"
[[link]]
from = "app"
to = "engine"
reason = "needed"
[[link]]
from = "app"
to = "adapter"
reason = "needed"
[[complexity]]
cognitive = 10
"#;

use std::path::PathBuf;

fn cfg(w: &World) -> PathBuf {
    w.save(&with_bricks(RULES))
}

#[test]
fn short_format_lines() {
    let w = world(|_| {});
    let o = run(&cfg(&w), &["--format", "short"]);
    assert_eq!(o.code, 1);
    let line = o
        .stdout
        .lines()
        .find(|l| l.starts_with("error[complexity/cognitive] "))
        .unwrap_or_else(|| panic!("{}", o.stdout));
    let mut p = line.splitn(3, ' ');
    p.next();
    let id = p.next().unwrap();
    assert_eq!(id.len(), 8);
    let rest = p.next().unwrap();
    assert!(rest.starts_with("crates/eng/src/x.rs:5"), "{rest}");
    assert!(
        rest.contains(": `busy` has cognitive 20 (limit 10)"),
        "{rest}"
    );
    assert!(
        o.stdout.lines().last().unwrap().starts_with("error: "),
        "{}",
        o.stdout
    );
    assert!(!o.stdout.contains("-->"));
}

#[test]
fn measure_flag_parses_and_skips_a_fresh_cache() {
    let w = world(|_| {});
    let c = cfg(&w);
    let o = run(&c, &["--measure", "--format", "json"]);
    assert_eq!(o.code, 1, "{}{}", o.stdout, o.stderr);
    assert!(!o.stderr.contains("wrote"), "{}", o.stderr);
}
