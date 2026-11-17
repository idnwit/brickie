//! Check tests: text output.

use super::support::*;

#[test]
fn text_output_snapshot() {
    let w = world(|_| {});
    // Map the unmapped module so only the two intended findings remain.
    let cfg = w.save(&format!(
        "{}\n{}",
        bricks_with_misc(),
        r#"
[[rule]]
from = "*"
to = "*"
reason = "open"

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
bricks = "tag:core"
cognitive = 15
"#,
    ));
    let o = run(&cfg, &["--color", "never"]);
    assert_eq!(o.code, 1);
    let path = cfg.display();
    let expected = format!(
        "error[complexity/cognitive]: `busy` has cognitive 20 (limit 15)
 --> crates/eng/src/x.rs:5:8
  |
5 | pub fn busy() {{}}
  |        ^^^^ cognitive over the limit by 5
  |
  = rule: complexity[0] cognitive <= 15 ({path}:43)
  = help: return early or extract a function to cut cognitive by at least 5

error[deps/unlinked]: `engine` (core) uses `adapter` (backend)
 --> crates/eng/src/x.rs:2:14
  |
2 |     let io = AudioIo::build();
  |              ^^^^^^^^^^^^^^ written `item` → defined in `adp::y`
  |
  = rule: no link covers it ({path})
  = help: remove the dependency, or declare the link: `brickie design link add engine adapter --kind uses --scope prod --reason \"...\"`
  = note: modules: eng::x -> adp::y (uses, prod)

error: 2 errors
"
    );
    assert_eq!(o.stdout, expected);
    // Extra locations are summarised after three.
    let w = world(|c| {
        let e = c["files"]["crates/eng/src/x.rs"]["edges"]
            .as_array_mut()
            .unwrap();
        for l in 4..=9 {
            e.push(edge("eng::x", "adp::y", "uses", "prod", l));
        }
    });
    let cfg = w.save(&format!(
        "{}[[rule]]\nfrom = \"*\"\nto = \"*\"\nreason = \"open\"\n",
        bricks_with_misc()
    ));
    let o = run(&cfg, &["--color", "never"]);
    assert!(
        o.stdout.contains("= note: 6 more location(s): crates/eng/src/x.rs:4:14, crates/eng/src/x.rs:5:14, crates/eng/src/x.rs:6:14 and 3 more"),
        "{}",
        o.stdout
    );
    // Colour is only used when asked for.
    let o = run(&cfg, &["--color", "always"]);
    assert!(o.stdout.contains("\x1b[1;31merror[deps/unlinked]"));
    assert!(!run(&cfg, &["--color", "never"]).stdout.contains('\x1b'));
}

/// The fixture bricks with the otherwise unmapped `misc` crate in `app`.
fn bricks_with_misc() -> String {
    BRICKS.replace("members = [\"app\"]", "members = [\"app\", \"misc\"]")
}
