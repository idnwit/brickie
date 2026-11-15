# brickie tool issues found while designing this fixture

- No way to start a design from nothing. `brickie change open` needs an existing config:
  `brickie change open --intent x` -> `brickie: error: read brickie.toml: no such file` (exit 2).
  Workaround (manual edit): create an empty file with `: > brickie.toml`, then open the change.
  Suggest: `change open` (or `brickie init`) creates an empty `brickie.toml` when it is missing.
- Deleting `brickie.toml` while a change is open makes the change unpublishable, and the only way
  out is `brickie change discard`, which loses all ops (21 here; they had to be replayed):
  `brickie change publish` -> `change/outside-edit: brickie.toml was edited outside brickie since the change was opened`.
  Suggest: an option to rebase/reopen the op log on the current files.
- `brickie change publish --dry-run` before any `brickie measure` plans to scaffold wrong crates
  (`create domain/Cargo.toml (crate of brick domain)`, `create usecases/Cargo.toml ...`) for bricks
  whose crates already exist, because the cache is missing. Only the `--todo` list says
  `read .../.brickie/cache.json (run brickie measure first)`. Suggest: dry-run/publish refuse or
  warn loudly without a cache.
- `brickie serve` with the default port already taken fails with
  `Error: bind 127.0.0.1:7881 ... Address already in use (os error 98)`; no hint to use `--port`.
- Root rules cannot target a child of a composite. To allow `ui -> ports::inbound` but deny
  `ui -> ports::outbound`, `application` was flattened into three root bricks (`ports-in`,
  `ports-out`, `usecases`) instead of one composite with `exports`. The docs imply this (edges are
  checked at the lowest common ancestor) but do not say how to express per-child permissions.
- The repo `.gitignore` ignores only `/fixtures/*/target/`; `.brickie/` caches of fixtures are not
  ignored (a fixture-local `.gitignore` was added).
- Issue 5 (root rules cannot target a child of a composite) is NOT resolved in practice: `via` is
  accepted by `change propose` and shown in the change diff (`+ via = ["ports-in"]`), but after
  `change publish` the written `brickie.toml` has no `via` on the `ui -> application` and
  `adapters -> application` rules (they are plain allows). Result: `ui -> ports-out` and
  `adapters -> ports-in` pass `brickie check` (exit 0) where they must fail. `ui -> usecases`
  correctly fails with `deps/private`. Suggest: publish must write `via`; `check` of a published
  design should be verified by a round-trip test. Re-run the via cases after the fix.
- `brickie interface <brick>` needs a path such as `application/ports-out`; a bare child name
  gives `no brick` (the same child names are accepted in `via` and `exports`).
- Accepted op #2 of the via change targets the wrong rule. `set_rule` with `"index": 4` replaced
  `ui -> domain kinds=["uses"]` (index 4) instead of `ui -> application` (index 3), so the diff shows
  `- to = "domain"` / `+ to = "application"` and the design ends with two `ui -> application` rules
  and no `ui -> domain` rule. Fixed by proposing op #6 (`set_rule` index 3 back to the uses rule).
  Suggest: `set_rule` should carry the old rule's `from`/`to` (or a stable rule id) and refuse a
  mismatch; the diff could warn when a set_rule changes `from` or `to`.
- `brickie change publish --dry-run` with proposed ops prints only `write brickie.toml (config)`
  followed by one `blocked: change/proposed: op N ... is still proposed` line per op (47 here). The
  plan for the proposed ops (new `crates/domain/brickie.toml`, `crates/ui/brickie.toml`) is not
  shown, so an agent cannot preview what its own ops will scaffold. Suggest: a `--include-proposed`
  preview, and one summary line instead of 47 blocked lines.
- `make_composite` on a leaf brick with `members` (`domain`: `members = ["todo-domain"]`) keeps
  `members` next to the new `path` (diff shows only `+ path = "crates/domain"`). Unverified whether
  check accepts a composite with members; `set_brick` has no way to clear `members` (empty list
  is not documented as removal, unlike `desc`/`path`).
- Human op #5 (`set_brick domain tags = ["adapter"]`, accepted) retags the domain as an adapter;
  with `tag:core -> tag:adapter deny` this likely flags `application -> domain`. Left untouched
  (human op); flagged for review.
- `brickie measure` prints `ERROR Bug: definition at crates/domain/src/column.rs:48:16-48:21 should have been in an SCIP document but was not.` (4 lines for `todo-domain`); the cache is still written and `check` is clean. Source of the lines is unclear to the user.
- `brickie design brick set X --path P` on a module brick fails (`config/missing`: no `P/brickie.toml`), so a leaf cannot become a composite; workaround: remove its rules and brick, then `brick add X --kind group --path crates/X`. `brick add --kind group` without `--path` creates `X/` at the workspace root, not `crates/X/`; `brick group` does the same with the new name.
- Allow rules default to `scope = "prod"`, so test code in a brick fails with `deps/default` ("in tests"); the help text does not mention `scope = "all"`.
