# Check

Scope: `brickie check`: config tree, member resolution, rules, interfaces, findings, `llms.txt`. Back to [../design.md](../design.md).

Example config: `docs/examples/brickie.toml`. Code: `crates/core/check`. `check` reads the cache
and configs and resolves the brick tree on every run. It needs no rust-analyzer. See [measure.md](measure.md).

## Config tree

- Each `brickie.toml` plus includes is one level. A brick with `path = "<dir>"` is a composite.
  Its level is `<dir>/brickie.toml`; `members` default to `path:<dir>/`.
- Names, tags and selectors are local to their file. `include` is recursive, merged before the
  including file, and a file reached twice merges once; a cycle is an error. A `path` that leads
  back to a file being loaded is an error.
- Lists concatenate; no scalar overrides (only `[measure]`, root-only). Schema and the model
  rules: `docs/llms.txt` ("brickie.toml reference", "Rules").
- Unknown fields, duplicate bricks and unknown selectors are errors (exit 2).
- Small schema mistakes (unknown selector, kind or export, nested `independent`) share one code,
  `config/invalid`, with a message naming the fix: one code per kind of fix, not per key.
- `via` is validated after the composites load (`config/unknown-via`): `to` names a composite brick, the list is not empty and every name is in that composite's `exports`. `via` is a property of a rule, not of its identity: `rule set` keeps it unless given. An unused-link check backs a `via` rule only with a measured edge that enters through a `via` child. Design ops keep it valid: renaming a child rewrites parent rules; remove, move, unexport and ungroup fail with `design/invalid` when a `via` would dangle.
- Per-level semantics (exports, partition): [levels.md](levels.md).

## Resolution: member selectors

Each module gets one leaf path. At each level a module matches the members of one brick only; two is `config/overlap` (llms.txt rule 2).
Decision: one length scale instead of the old 4-tier specificity and the exact `module:`
selector. Reason: a single ordering is easier to predict, and a module prefix already selects a
subtree; an equal length in two bricks stays an error.

- An unknown or inactive module ID stays unmatched.
- A `path:` selector is normalised lexically (no file system access) against the declaring
  file's directory. A member equal to a package name is a package before it is a module id.
  Equal members of one brick never conflict.
- Resolution descends: a module given to a composite is assigned again among its children, and
  the leaf path is where it stops.
- Below a composite, children must partition its modules: `config/unassigned`,
  `config/member-outside-parent`. At the root an unmatched module is `bricks/unmapped` (an error:
  it would escape every rule);
  it takes part in no rule, cycle or complexity check, but its references still count as uses
  for `interface/unused`.
- A composite owns the modules under its path. A brick of an outer level that matches one is
  `config/overlap` at that level (no longest-match tie-break, so nothing can bypass `exports`);
  the fix is to make the module a child brick of the composite and export it if outside code
  uses it.
- Resolution is strict and has one path: it collects every partition error (an overlap goes to
  the first brick, an unassigned module stays unmapped, an outside member is ignored, so later
  errors are still found) and then fails with all of them. `check` never reads the design draft;
  `brickie design publish --dry-run` resolves the draft the same way and lists the errors as its
  checklist ([editor.md](editor.md#draft-and-publish)).

## Rules

The rules themselves are stated once in `docs/llms.txt` ("Rules"). Decisions behind them:

| Decision | Reason |
| --- | --- |
| One decision function, `config::decide(level, from, to, entered, kind, scope)` (`entered`: the child of `to` the edge enters through, for `via`), implements rules 5 and 7; `check`, the design editor (`link_options`, `link_check`, notes), the unused-link list and the viewer all call it | a rule re-implemented in a second place drifts |
| No match is deny, fixed (`deps.default` cut); `* -> *` allow opens a level | the design lists what is allowed; one fewer option per file |
| Every forbidden edge is `deps/deny` (`deps/default`, `deps/independent` merged into it) | one fix per code: remove the edge or ask for a rule |
| Sibling cycles are always errors (`deps.cycles` cut) | a layered design has no cycles |
| Exports are a plain list (scoped exports cut); `via` kept | `via` narrows a rule to an exported entry point (ports in a hexagon); scoped exports duplicated what scope on rules does |
| Rule `help` and complexity `help` / `level` cut | every finding is an error; the fix text is per code in llms.txt |
| `independent`: `*` never matches it; inbound allowed unless a deny names it; outbound only to another independent brick under an allow naming both; an allow to a normal brick never applies (a `rule add` note says it changes nothing) | a library knows nothing of its users; no special code or config error needed |
| Unused allow rules are design todos only (`deps/unused` cut as a finding); a rule is used when `decide` picks it for a measured edge | an unwritten edge is planned work, not a violation |

Other rule facts: an edge lifts to the lowest common ancestor `L` (O(depth)); `deps/private` is
checked for every lifted edge and is independent of rules; one finding per module edge (from, to,
kind, scope); cycles are SCCs over lifted prod edges, one error per SCC; staleness candidates come
from `git diff` / `git status` confirmed by sha256, workspace members unknown to the cache are
`stale/file`, a cache measured elsewhere is `stale/measure`.

## Interfaces

- A leaf declares the cached items that code outside the leaf can name, or that a module of
  another leaf can name (a `pub(crate)` item shared between folder bricks). `interface/unused`
  works per leaf: a composite has no interface findings of its own, and exports do not widen it.
- Uses come from reference edges (not `reexports`) from outside the brick or from unmapped
  modules, or from test code anywhere. Production edges inside the leaf never count. A `#[test]` in a `mod tests` child counts; one in the item's own module does not.
- Not items: `macro_rules!` (no `pub` signature from rust-analyzer), a `pub fn` in a private
  module. A `pub(crate)` item shared between folder bricks is an item.
- Agents run `brickie interface <brick> [--json] [--from <brick>]`.

## Findings and exit codes

- A finding is a `Diagnostic` (`crates/core/check/src/diag/`). Its `id` is the first 8 hex of
  sha256 of `code\0subject`, so review decisions survive re-runs. Codes: `crates/core/model/src/codes.rs`.
- Id subjects (what follows `code\0`): `deps/deny|private` `from->to:kind:scope` by
  module; `deps/cycle` member names (prefixed by the level below root); `complexity/m` `file::fn::m`; `interface/unused`
  `leaf path:module:symbol`; `stale/file` the file; `stale/measure` `measure`; `bricks/unmapped`
  the module id.
- Text output is cargo-style. JSON is `{summary, diagnostics}`; the webview consumes it.
- `check --brick PATH` (repeatable) shows findings whose owner is the brick or a descendant,
  matched by level path. Owners: source brick of `deps/deny|private`, all members of
  `deps/cycle`, the defining brick of `interface/*`, the function's
  brick of `complexity/*`; ownerless findings (`stale/file`, `bricks/unmapped`) always show.
  Everything is evaluated; the exit code uses the shown set; the footer is `outside scope: N
  errors`. JSON adds `outside_scope` when used. Unknown bricks exit 2.
- Every finding is an error: no warning level, no accept file, no baseline, no skip list. To pass,
  fix the code or let a human relax the design (rules, limits). Output: `error: N errors` (or
  `ok: 0 errors`); JSON `summary` is `{errors}`.
- Exit 1: a finding is shown. Exit 2: config, cache or argument error. Else 0. Findings hidden by
  `--brick` never change it.
- The cache is `.brickie/cache.json` beside the config; a missing, unparsable or wrong-schema
  cache exits 2. `stale/measure` is judged only when the cache records its measure settings.
  `stale/file`, `stale/measure` and `bricks/unmapped` have no owner.

## Gate for coding agents

Decision (two independent reviews, then tests on a fixture and on this repo): `check` must be
usable as the only signal of a hook or CI step, so exit 0 means "checked and not worse".

| Hole found | Decision |
| --- | --- |
| Code edited after `measure` passed | `stale/*` is an error; `check --measure` re-measures when the cache is missing or stale |
| A new crate or module in no brick passed | `bricks/unmapped` is an error (like every finding) |
| `cfg(any(test, ..))`, `feature = "test.."` and a prod folder `src/tests/` counted as test scope | scope is test only for `cfg(test)`, `test` under `all(..)`, and Cargo's `tests/` and `benches/` ([measure.md](measure.md)) |
| Text output costs about 6 lines per finding | `--format short` |
| A project directory copied with its `.brickie/` was checked against the old location | the cache records where it was measured; another location is `stale/measure` |
| A new workspace crate was not stale, so `--measure` did not re-measure and the crate went unchecked | workspace members unknown to the cache are `stale/file` |
| Old facts that break the config (`config/unassigned` for a removed crate) blocked `--measure` | `--measure` judges staleness from the cache alone, before the config loads |

Limits, open:

- References to external crates are not edges, so "domain must not use an HTTP client" cannot be
  written. Proposal: bricks for external crates.
- `check --brick` exits 0 with errors outside the scope (by design, for mid-work); a gate runs
  the full check.
- `interface/unused` cannot be fixed for a `pub(crate)` item of a crate root module that other
  modules of the same brick use: it is the narrowest visibility that compiles, yet sibling folder
  bricks can name it, so it counts as interface (such findings remain possible). A use
  from outside Rust (a `#[wasm_bindgen]` export called from JavaScript) is not seen either.
  Proposal: count a use from the same brick when the item is not `pub`.

## `brickie llms.txt`

Prints [../llms.txt](../llms.txt) (embedded, reads no project): the one guide for coding agents. It
holds the `measure -> check -> fix -> re-check` workflow, every diagnostic code with its fix,
the config schema and "Editing the design". An agent never edits rules to pass. The test `guide_documents_every_diagnostic_code` fails when a code is missing.

## Open items

- Config errors without a code: duplicate brick, include cycle, path cycle and unknown fields
  exit 2 but have no `config/` code. Proposal: report them as `config/invalid`.
