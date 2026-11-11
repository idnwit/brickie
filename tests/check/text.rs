//! Check tests: text output.

use super::support::*;
use serde_json::json;

#[test]
fn text_output_snapshot() {
    let w = world(|c| {
        // Map the unmapped module so only the two intended findings remain.
        c["bricks"]["module_brick"]["misc::q"] = json!("app");
    });
    let cfg = w.save(&with_bricks(
        r#"
[[deps.rule]]
from = "tag:core"
to = "tag:backend"
action = "deny"
reason = "core must not use adapter"
help = "depend on the port and receive the impl from the assembly"

[[complexity]]
bricks = "tag:core"
cognitive = 15
level = "warn"
"#,
    ));
    let o = run(&cfg, &["--color", "never"]);
    assert_eq!(o.code, 1);
    let path = cfg.display();
    let expected = format!(
        "error[deps/deny]: `engine` (core) uses `adapter` (backend)
 --> crates/eng/src/x.rs:2:14
  |
2 |     let io = AudioIo::build();
  |              ^^^^^^^^^^^^^^ written `item` → defined in `adp::y`
  |
  = rule: deps.rule[0] \"core must not use adapter\" ({path}:18)
  = help: depend on the port and receive the impl from the assembly
  = note: modules: eng::x -> adp::y (uses, prod)

warning[complexity/cognitive]: `busy` has cognitive 20 (limit 15)
 --> crates/eng/src/x.rs:5:8
  |
5 | pub fn busy() {{}}
  |        ^^^^ cognitive over the limit by 5
  |
  = rule: complexity[0] cognitive <= 15 ({path}:25)
  = help: return early or extract a function to cut cognitive by at least 5

error: 1 error, 1 warning; 0 accepted
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
        c["bricks"]["module_brick"]["misc::q"] = json!("app");
    });
    let cfg = w.save(&with_bricks(
        "[[deps.rule]]\nfrom = \"engine\"\nto = \"adapter\"\naction = \"deny\"\n",
    ));
    let o = run(&cfg, &["--color", "never"]);
    assert!(
        o.stdout.contains("= note: 6 more location(s): crates/eng/src/x.rs:4:14, crates/eng/src/x.rs:5:14, crates/eng/src/x.rs:6:14 and 3 more"),
        "{}",
        o.stdout
    );
    // Colour is only used when asked for.
    let o = run(&cfg, &["--color", "always"]);
    assert!(o.stdout.contains("\x1b[1;31merror[deps/deny]"));
    assert!(!run(&cfg, &["--color", "never"]).stdout.contains('\x1b'));
}
