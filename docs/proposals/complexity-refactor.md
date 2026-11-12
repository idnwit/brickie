# Proposal: split complex functions into libs and wiring flows

Status: **in progress** (started 2026-11-12). Steps are checked off here as they land.

## Principle

Code is only:

- **Libs**: logic with a simple, explicit contract (small pure functions or types; may be
  internally complex; reusable or private to their crate). Tested thoroughly, branch by branch.
- **Wiring flows**: linear orchestration that calls libs, no decisions of its own (cognitive
  about 8 or less). Needs almost no tests.

Decision logic found inside a flow is pulled out into a lib. No new crates and no new dependency
edges: libs stay in their own algo crate (`crates/lib/algo/*`, which may not call each other) or
become private modules of `crates/core/measure`.

## Test placement

- **Unit tests**: same file as the code, in a `#[cfg(test)] mod tests` at the bottom, so they can
  reach private items; visibility is never widened for tests. Only for simple-contract functions
  and structs (inputs and outputs stateable in one doc-comment line).
- **E2e tests**: a test that goes through a brick's public interface is an e2e test. It lives in a
  separate file or an `e2e_tests/` directory (golden snapshots included). Complex orchestration is
  covered this way, not by unit-testing its private pieces one by one.
- Tests are exempt from the file size guideline.

## Targets

Hard ceiling (`bricks = "*"`, error): cognitive 60, lines 200. Domain (warn): cognitive 15, nesting 4,
params 6, lines 80. UI functions are out of scope for now.

| # | Function | Today | Libs after the split | Flow left |
|---|---|---|---|---|
| 3 | `measure_items` (`layout/src/measure.rs`) | cognitive 69 | `seg_cross`, `pair_crossings`, `box_hits`, `bends` | three sums |
| 1 | `smart_route` (`layout/src/smart.rs`) | cognitive 131, 255 lines | `smart/cand.rs` (`box_hits`, `side_penalty`, `candidates`), `smart/choose.rs` (`cost`, `greedy`), `smart/ports.rs` (`spread_ports`, `settle`) | candidates, order, greedy, settle, write result |
| 2 | `walk` (`metrics/src/cognitive.rs`) | cognitive 69 | `skip`, `child_nest`, one fn per node kind (`walk_if`, `walk_match`, `walk_nested`, `walk_logic`, `leaf_cost`) | skip, depth, dispatch by kind |
| 6 | `test_files` (`measure/src/run/files.rs`) | cognitive 35 | `mod_candidates` (pure string), `closure` (fixpoint) | seed, fixpoint |
| 7 | `Reach::new` (`measure/src/run/reach.rs`) | cognitive 23 | `pub_uses`, `public_closure`, `uses_by_symbol` | three calls, build struct |
| 5 | `add_edges` (`measure/src/run/edges.rs`) | cognitive 26 | `edge_kind`, `edge_endpoints`, `end_col` | loop: endpoints, build edge, insert |
| 4 | `normalise` (`measure/src/scip/normalise.rs`) | cognitive 66, 203 lines | `scip/files.rs` (`package_of`, `file_row`), `scip/defs.rs` (`collect_defs`, `symbol_row`), `scip/tree.rs` (`assign_parents`, `file_modules`, `name_crate_roots`, `link_modules`), `scip/occ.rs` (`resolve`, `use_at`, `occurrences`) | about 8 sequential calls returning the `Graph` |

## Steps (sequential, each green before the next)

Green = `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. No behaviour change in any step unless stated.

1. [x] Characterization tests for `measure_items`, `smart_route`, `walk` (golden values captured
   from the current code; floats printed with `{:.3}`).
2. [x] Test builder (`#[cfg(test)]` hand-built `Graph`/`Index`) and characterization tests for
   `test_files`, `Reach::new`, `add_edges`, `normalise`.
3. [x] Split `measure_items`.
4. [x] Split `smart_route` (keep the update order of `choice` in `cost`/`greedy`).
5. [x] Split `walk`.
6. [x] Split `test_files`, `Reach::new`, `add_edges`.
7. [x] Split `normalise` (pure refactor; golden identical; repo cache identical for untouched files).
8. [x] Fix real defects (8a triage, 8b unreadable files fixed, 8c docs and pinned tests for the acceptable behaviours): triage every "Found, not fixed" item (an opus reviewer verifies each against the
   code and the intended contract: real bug, intended behaviour to document, or not reproducible).
   Each real defect is fixed in its own change with a failing test first (behaviour changes are
   deliberate and separate from the pure refactors), then its row moves to "Fixed" with before and
   after evidence. This includes the unreadable-file case (`normalise`, `analyze_all`,
   `add_uncompiled`). Intended behaviour gets a doc comment and a pinned test instead.
9. [x] Consolidate tests (layout: `tests/` plus `src/e2e_tests/`; metrics: `tests/`; measure: `src/e2e_tests/` for scip goldens and `src/run/e2e_tests/` for run goldens, since `run::edges`/`run::files` are private; shared builders in `scip/testutil.rs`, because brickie's own deps check rejected them under `e2e_tests` (scip tests would depend on run)): unit tests that steps 1-5 put in separate files (`layout/src/tests_measure_fns.rs`,
   `smart/tests_*.rs`, `measure/src/run/tests_*.rs`, `scip/tests_normalise.rs`, ...) move next to the
   code they test when they cover simple-contract fns; tests of complex flows through private fns
   are dropped when an e2e/golden test already covers them. Public-interface golden tests
   (`tests_smart*.rs`, `tests_measure.rs`, `tests_walk.rs`, `tests_normalise.rs`) move to
   `e2e_tests/` of their crate.
10. [x] Critique and condense (C1-C6 done, see "Condense results"): after the splits, two independent reviewers (opus) challenge the result
   with the sonnet authors and shorten it. Principles: prefer OOP (a small struct owning its state
   with methods and a clear contract) over loose functions passing many params; DRY (merge
   near-duplicate helpers, e.g. the several `box_hits`/`inside` variants); KISS (drop helpers that
   only rename one expression, avoid generic machinery where one concrete type will do). Keep the
   golden/e2e tests unchanged as the safety net; record the decisions here.
11. [x] Final `brickie check`: 0 errors; every function in the original target list is under its limits
    (the four hard-ceiling errors are gone). Raising the `tag:domain` rule to `error` is NOT done:
    29 domain complexity warnings remain, almost all in the UI (`node_view` 56, `evaluate` 49,
    `edge_view` 44, `canvas`, `pointer`, `Ctx::level`, `brick_index`, `functions_view`, `frame_edges`,
    `left`, `keys`, `render`), plus `complexity` (rules), `add_items` (items), `heat`, `assign_targets`,
    and 13 `lines` warnings. Next proposal: refactor these, then switch the rule to `error`.
12. [ ] (next) UI and remaining domain functions, then `tag:domain` to `error`; clear or accept the 44
    `interface/unused` warnings.

## Report: findings and benefits (updated after each step)

Status as of step 8 (splits done, the one real defect fixed, acceptable behaviours documented and pinned). Honest summary: the refactors so far are pure (no behaviour change, proven by
golden tests), so the benefit is controllability and coverage, plus the risks the golden tests
exposed. Items marked *found* are NOT fixed yet; only *fixed* items changed behaviour or config.

| Measure | Before | Now |
|---|---|---|
| `brickie check` errors / warnings | 0 / 82 (config was blind to `lines`, `params`, hard ceiling) | 0 / 83 (the ceiling is live; 4 hard errors reduced to 0 by refactor) |
| `measure_items` cognitive | 69 | 0 (helpers <= 12) |
| `smart_route` cognitive / lines | 131 / 255 | 7 / 33 (helpers <= 14) |
| `walk` cognitive | 69 | 3 (helpers <= 6) |
| `test_files` cognitive / nesting | 35 / 5 | 2 / 2 |
| `Reach::new` cognitive | 23 | 1 (helpers <= 7) |
| `add_edges` cognitive | 26 | 7 (helpers <= 5) |
| `normalise` cognitive / lines | 66 / 203 | 0 / 14 (a builder with one method per phase; helpers <= 7) |
| Direct tests | none on `smart_route`, `normalise`, `add_edges`, `test_files`, `Reach::new` | golden tests on all of them; unit tests for the extracted simple-contract fns (layout 84, metrics 11, measure 23 tests) |

### Fixed

- **Config was too loose** (2026-11-12): only `cognitive` and `nesting` were limited, every rule
  was `warn`, algorithms had no hard cap (`smart_route` at 131 was one warning among 82) and a
  brick without a tag was unchecked. Now: `lines` and `params` for domain, a root `*` ceiling as an
  error, and a test-scope rule.

- **Unreadable files** (8b, 2026-11-13): before, a file that could not be read was recorded as the
  sha256 of the empty string with `lines = 0`, and `analyze_all` also turned a non-UTF-8 file into
  an empty source, with no message. Now `measure` prints `warning: cannot read <path>; recorded as
  empty` once per path, and `analyze_all` analyses non-UTF-8 files lossily like `add_uncompiled`.
  Tests written first (`unreadable_paths_are_reported`, `analyze_all_reports_unreadable`); cache
  rows unchanged. Known limit: a non-UTF-8 compiled file is still hashed from an empty source in
  its cache row (kept to avoid a schema/golden change).

- **Watcher could miss a change** (found as a flaky test, 2026-11-13): `Change::open` writes
  `ops.json`, then `readers.json`, then `intent`; the web watcher cached "unreadable, not open"
  under the new `ops.json` stamp when a poll landed between the writes, so it never emitted
  `Change{open:true}` afterwards (`events/scan.rs`). `the_watcher_pushes_what_changed` failed 3
  times in about 12 full workspace runs. Fixed: an unreadable change is never cached (sentinel
  stamp), plus a deterministic test written first (`a_look_between_the_open_writes_does_not_hide_the_change`,
  failed before the fix) and the test now waits for the watcher's first snapshot instead of
  sleeping 400 ms. 6 of 6 full workspace runs green afterwards.

### Found, not fixed (risks the characterization tests pinned; step 8 triages and fixes the real ones)

| Where | What | Risk | Proposed |
|---|---|---|---|
| `normalise` (`scip/normalise.rs`) | a failed file read becomes sha256 of an empty string and `lines = 0`, silently; same in `analyze_all` and `add_uncompiled` | a wrong cache without any error | surface unreadable paths (open decision below) |
| complexity metrics (`walk`) | macro bodies are not parsed: branching inside `println!(..)`, `vec![..]` and `view! { }` is invisible | complexity of macro-heavy code (the viewer's `view!`) is under-reported | measure macro token trees, or document the blind spot in `docs/llms.txt` |
| complexity metrics (`walk`) | a closure adds nesting but no cost itself; labeled `break` costs 11 in a 3-deep loop with two `if`s | scores diverge from the SonarSource definition the docs cite | decide on the definition, then adjust and re-pin |
| `normalise` | a module def starting at (0,0) and a def sharing the same enclosing range become each other's `container`, e.g. the `util/` module def in `util.rs` has a child as container | edge sources may be attributed to a child instead of the module | check with `exact-module-members.md` |
| `add_edges` | a non-import occurrence of a `pub` use is a plain `uses`, not `reexports` | re-export edges can be missed | confirm intended |
| `Reach::new` | `reexported` also contains module symbols | readers must filter | document or split |
| `reach.rs` `module_scope`, `visible_in` | `pub(in path)` and `pub(self)` fall into the `_ => root` arm: widened to crate-wide | a narrowly scoped item looks wider than it is (conservative) | map the visibility forms explicitly |
| `reach.rs` `mod_vis` | `sig.split(" mod ")`: a signature with attributes or no space before `mod` reads as private | a public mod may be treated as private | verify what rust-analyzer emits; parse robustly |
| `reach.rs` `pub_uses` | a `use` in a file with no module and no container is silently dropped | a re-export may be missed | report instead of dropping |
| `edges.rs` `add_edges` | `analyses[&f.path]` and `analyses[path]` panic when `analyses` lacks a compiled file; `g.symbols[..]`, `g.files[..]`, `g.use_items[..]` index directly; unresolved endpoints and self-edges are skipped without a diagnostic | a panic instead of a diagnostic on an inconsistent graph; edges lost silently | return an error or diagnostic; count skipped edges |
| `scip/defs.rs` `collect_defs` | a symbol defined twice in one file keeps only the first row | a `cfg`-duplicated def is dropped silently | report or keep both |
| `scip/link.rs` | `registry.entry(..).or_insert` keeps the first module per (target, path); files with target `None` share one key; `ws.targets[t]` panics when `file.target` is out of range | wrong parent for duplicate module paths; a panic on an inconsistent graph | detect duplicates; check the range |
| `scip/defs.rs` `resolve` | `None == None` counts as "same target" for files without a target | a def from an unrelated untargeted file can win | require `Some` for the same-target preference |
| `scip/files.rs` `package_of` | a package dir with a trailing slash never matches | a file falls out of its package | normalise the dir |
| `smart_route` ports | top/bottom ports are sorted without an id tie-break (left/right break ties by id) | route order can depend on input order | add the tie-break and re-pin the goldens |
| `smart_route` capacity | old routes count against a side's capacity but are never refused | a side can end over capacity | document or enforce |
| `brickie measure` | rust-analyzer logs `should have been in an SCIP document` for many definitions and `Duplicate symbol` for several crates (before this work) | some definitions may be missing from the cache | investigate separately |

### Triage (2026-11-13, opus reviewer, evidence by reading code and fixtures)

Of the 17 rows above: **1 real defect (A)**, 10 acceptable behaviour to document and pin (B), 6 not
reproducible or unreachable (C).

- **A, fixed in 8b:** unreadable files are recorded silently (`scip/files.rs`, `run/files.rs`).
- **B, documented and pinned in 8c:** macro bodies unparsed (the doc in `metrics/src/lib.rs` is
  right, `docs/llms.txt` was silent); closure and labeled `break` scoring (this matches SonarSource,
  so the earlier row was wrong); `container` of a module def (edges drop defs, output unaffected);
  `reexported` holding module symbols; `pub(in ..)` and unknown visibility widened to crate-wide on purpose (`pub(self)` and `pub(super)` have their own arms in `module_scope`; for items only `pub` is narrowed in `visible_in`; corrected by the 8c implementer); `pub_uses`
  skipping files outside the module tree; a def twice in one file (rust-analyzer skips inactive
  `cfg`); `(None, path)` module key; `None == None` same-target (candidates always share a
  package); `smart_route` top/bottom ports without an id tie-break (deterministic input order and
  stable sort); port capacity (documented).
- **C, closed:** a `pub` use that is not an import (cannot occur: `use_item` only exists for
  imports, `scip/occ.rs`); `mod_vis` `split(" mod ")` (rust-analyzer signatures are `vis mod x`);
  `add_edges` panics (`analyses` is built from `g.files`, so every key exists); `ws.targets[t]`
  range (indexes come from `enumerate`); `package_of` trailing slash (dirs come from
  `Path::parent`); rust-analyzer log noise (outside this code; investigate separately).
- **Optional, needs a user decision:** an id tie-break for top/bottom ports would change golden
  offsets for equal keys. Not done.

### Condense plan (step 10, from two independent opus critiques, 2026-11-13)

Each step is one sonnet agent, goldens byte-identical, free to REJECT a proposal with evidence
(recorded below). Expected saving about 160 production lines.

| Step | Scope | Proposals |
|---|---|---|
| C1 | layout geometry | `Rect::contains` (shared by `measure.rs` and `smart/cand.rs`), `Rect::side_len`, one `bb_apart(a, b, margin)` replacing `bbox_apart`, the inline bbox test and `far_apart` |
| C2 | `smart/` | `Cand::new` computes `len`/`bb`/`cp`/`boxh` itself; one `direct` cubic helper (params <= 6); inline `offset`/`step`; `placements` as a loop over both ends; `sort_side` without a `String` clone per compare |
| C3 | `metrics/cognitive.rs` | drop `is_branching`, inline `walk_closure`, merge `walk_if`'s loop into `walk_blocks`, `skip` with `any`; no public API change |
| C4 | measure DRY | shared `from_module(g, ids, o)` (edges and reach) and drop `Endpoints`; one definition-role check (`is_def`); one `descriptor_names` for `module_path` and `symbol_name`; `is_local` |
| C5 | measure KISS | drop the generic `closure` fixpoint in `test_files`; inline `Public::closure`; share the base-dir computation between `expected_module_path` and `mod_candidates` only if a test proves equal results |
| C6 | `normalise` | merge `files.rs` and `occ.rs` into `normalise.rs`, `tree.rs` into `link.rs`; phases as `Builder` methods; then drop `use_base`/`add_use_items` if order is unchanged |
| not now | | a `Walker` struct and `Item::new` (public API churn, low value) |

### Condense results (C1-C6, 2026-11-13)

All steps verified green (fmt, clippy, tests, `brickie check` 0 errors); goldens untouched.
Honest balance: the condense saved less than estimated because DRY helpers also add a fn and a
test; the main wins are fewer duplicated formulas and fewer files, not raw line count.

| Step | Applied | Rejected (with evidence) | Production lines |
|---|---|---|---|
| C1 layout geometry | `Rect::contains`, `Rect::side_len`, `Rect::bb`, one `bb_apart` (equivalence table incl. NaN and -0.0) | none | `measure.rs` 421 to 409 (file totals incl. tests) |
| C2 `smart/` | `Cand::new` derives `len`/`bb`/`cp`, one `direct` cubic helper, `spread_ports` inlines `offset`/`step`, `placements` loop, `sort_side` without clone | none (`mod.rs` map is a wash: +2 lines; `cand.rs` +11, `ports.rs` -22) | about -9 net |
| C3 `cognitive.rs` | dropped `is_branching`, inlined `walk_closure`, merged `walk_if` loop into `walk_blocks`, `skip` with `any` | a private struct for `walk`'s 6 params (ripples through `Acc`) | 208 to 177 |
| C4 measure DRY | `from_module`, one `is_def`, `descriptor_names`, `is_local`, one `file_module` per file | none | `edges.rs` about -20, `reach.rs` -4, helpers +25 (new shared fns) |
| C5 measure KISS | plain loop instead of generic `closure`, `Public::closure` inlined | shared `mod_base`: the two base-dir computations differ on 5 inputs (`a/lib`, `a/b/main`, `mod`, `lib.rs.rs`, `a/mod.rs.rs`); unreachable in practice but merging would change behaviour | `files.rs` 225 to 213, `reach.rs` 214 to 204 |
| C6 `normalise` | 5 files to 3 (`normalise.rs`, `defs.rs`, `link.rs`), phases as `Builder` methods, `use_base` replaced by ranges into `g.use_items` (order identical, cache `use_items` identical) | none | `normalise.rs` 64 to 150 (absorbed three files) |

Net `scip/`: 5 phase files to 3. The reviewers' estimate of about 160 lines was optimistic.

## Decision: unreadable files

Decided 2026-11-13: real defects are fixed (step 8), so this one is included.
`normalise` hides a failed file read (`source(&path).unwrap_or_default()`: sha256 of an empty
string and `lines = 0`); `analyze_all` (`files.rs`) and `add_uncompiled` do the same. Proposed:
return the unreadable paths and warn through `progress`. Not part of the pure refactor; it is fixed as its own
change in step 8b. Chosen design: warn through `progress` and keep the cache row as today (no
schema or golden change); `analyze_all` reads bytes with lossy UTF-8 like `add_uncompiled`.

## Evidence

Two independent reviews (config audit; complexity ranking) and one split plan, all on 2026-11-12.
`normalise` ranked first by blast radius (every later stage reads its graph) and has no direct
test; `smart_route` is the largest but only affects drawing.
