# brickie feedback (hexagonal-todo)

Written while the main agent designed the architecture with `brickie design` and Sonnet subagents
implemented domain, application, adapters, ui and app, one after another (no parallel runs).

## What brickie helped with

| Benefit | Evidence |
|---|---|
| Caught a real design flaw before it spread | `deps/cycle` `column -> kernel -> column`: the design had `DomainError` naming `ColumnId` while `column` used the errors. We moved the ids into the kernel and updated `DESIGN.md`. |
| Gave subagents an objective gate | Each agent had to end at `brickie check` exit 0; the main agent re-ran `measure` and `check` itself, so "done" was checked, not claimed. |
| `via` works end to end | Probe: `adapters` referencing `ports::inbound` failed with `deps/default ... via board-service`; reverting the probe gave exit 0. |
| `interface` showed API leakage | `brickie interface application/board-service` and `domain/column` let us confirm that no storage or note/board types leaked. |
| `interface/unused` kept the API tight | It flagged `board_view` as public but unused; the fix was `pub(super)`. |
| `design` log, `--base` and undo made edits safe | A bad `brick group` was reverted with `design undo`. |

## Issues

| # | Issue | Impact | Suggestion |
|---|---|---|---|
| 1 | `brickie measure` prints `ERROR Bug: definition at ... should have been in an SCIP document but was not` (3 to 11 lines per run, always on tuple-struct definitions). The cache is still written and `check` is clean. | Alarming noise; every agent asked whether it mattered. | Fix the indexer mapping, or downgrade and explain it once. |
| 2 | An `allow` rule defaults to `scope = "prod"`, so unit-test code in the same brick fails with `deps/default ... in tests`. | Six false-looking errors on correct code. The help text only says "add a `[[deps.rule]]`" and never mentions `scope = "all"`. | Mention `scope` in the `help` line of test-scope findings; `rule add` could default to `all`. |
| 3 | `brick set X --path P` cannot turn a module brick into a composite (`config/missing`: no `P/brickie.toml`). | We had to delete the rules and the brick, then re-add it as a group. | Let `brick set --path` create the nested file, as `brick add --kind group` does. |
| 4 | `brick add --kind group` without `--path` creates `X/` at the workspace root, not `crates/X/`. `brick group` also creates a folder at the root. The llms.txt says the default members are `path:crates/<name>/`. | We wrote `domain/` and `domain-g/` in the wrong place and fixed them by hand. | Make the default path match the documented one, or print the chosen path. |
| 5 | Deleting a scaffolded file or folder made every `design` command fail (`config/missing`), including `show` and `undo`. | We had to recreate the file by hand before `undo` worked. | `design show` and `undo` should still run on a broken tree. |
| 6 | `interface/unused` says to use `pub(super)`, but an unused Props struct still warns when it is `pub(super)`. | The only way to clear the warning was to delete the struct. | Do not suggest narrowing when the item would still be unused, or skip items private to one module. |
| 8 | Limits of static checking: a wasm panic (`unreachable`, signals set after the board re-rendered and disposed them) passed `measure`, `check`, clippy and all unit tests, and was only found by running the app in headless Chrome. | A green `brickie check` says nothing about runtime behaviour; the layering was fine and the app still crashed on the first form submit. | State this scope in the docs; pair `check` with a browser smoke test. |
| 7 | Allow rules to modules that are not reachable yet appear as `design/planned-edge` todo lines (about 17 for this design). | Long todo list while code is still being written. | Group them or add `--no-planned`. |

## Process notes

- Subagents obeyed the "fix the code, never the config" rule. Two real design gaps (issues 1 and 2) were
  reported to the main agent instead of being worked around, which is the behaviour we wanted.
- `deps/private` on the last step (`app` uses `application::usecases`, which was not exported) was a real
  design gap, reported by the app agent; the main agent added the export with `brickie design export set`.
- Negative probes (each reverted afterwards, `check` back to exit 0), all failed `brickie check` as required:

  | Probe | Finding |
  |---|---|
  | `adapters -> ports::inbound` | `deps/default ... via board-service` |
  | `ui -> ports::outbound` | `deps/default ... via board-storage` |
  | `usecases::columns -> usecases::notes` | `deps/default` |
  | `domain::column -> note` | `deps/cycle` and `deps/default` |
  | `ui::drag -> note` | `deps/cycle` and `deps/default` |

- Final state: `brickie check` 0 errors and 0 warnings; fmt, clippy `-D warnings`, `cargo test --workspace`,
  wasm32 build and the native smoke run pass.
- Smaller notes: the `deps/private` message points at `exports` but not at the CLI command
  (`brickie design export set`); an agent without the llms.txt would not know it.

## Browser verification (headless Chrome via Playwright)

Found and fixed with the check gate staying green throughout:

| Finding | Fix |
|---|---|
| Panic `unreachable` on every form submit (disposed signal set after the refresh bump) | `batch(...)` in the form handlers and `Ctx::report` |
| `.drop-zone` had zero height, so notes could not be dropped there | `min-height` in `style.css` |
| `storage` build restarted ids at 1, so new ids could collide with saved ones | `SequentialIds::starting_after`, seeded from the stored maxima in `wiring.rs` |
| No UI to reorder columns | "←"/"→" buttons with a pure `shift_target` and unit tests |
| Add-column form clipped at the right edge | `.board { flex-wrap: wrap }`, `.column-form { min-width }` |

Checked in the browser: add, edit and delete columns and notes, validation messages, the refused delete of a
non-empty column, drag between columns (closed state follows `is_close`), drag within a column, column
reorder, and persistence across reload with `--features storage`. Not checked: touch input, other browsers,
keyboard accessibility.

## Update with the newer brickie

| Earlier issue | Now |
|---|---|
| SCIP `ERROR Bug` noise | gone |
| Test-scope `deps/default` help lacks `scope` | help names ``rule add ... --scope all`` |
| `brick set --path` on a module | works |
| Broken tree breaks every `design` command | `design show` warns and `undo` restores |
| Unused allow rules invisible to `check` | `deps/unused` warning (named bricks only; `*` rules are not reported) |
| `cfg(target_arch)` code invisible (`app -> ui` unmeasured) | `[measure] additional_targets` plus a `hint:` line; verified: `mount` is indexed and `deps/unused` for `app -> ui` disappears |
| `brick add --kind group` default path | still `<name>/` at the workspace root |
| `planned-edge` one line per rule | unchanged |

Applied to this fixture: `[measure] additional_targets = ["wasm32-unknown-unknown"]` and explicit `app` rules
(`application`, `adapters`, `ui`, `domain`) instead of `app -> *`. `[measure]` has no `brickie design` command,
so it was added by hand and recorded as an `external` version.
