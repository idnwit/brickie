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
- A link naming `composite/child` is validated after the composites load (`config/invalid`): the target is a composite and the child is in its `exports`. The rest of the rules and links decision: [links.md](links.md).
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
- Resolution has one path and never fails on structure: every partition error and a missing
  nested `brickie.toml` (`config/missing`; the brick stays a leaf) is a finding (an overlap goes
  to the first brick, an unassigned module stays unmapped, an outside member is ignored), and
  the tree is always returned. Only unreadable or unparsable files and include or `path`
  cycles stop the load. `check` resolves the working design (the draft, else the published
  tree) ([editor.md](editor.md#working-design)).

## Rules

The rules and links themselves are stated once in `docs/llms.txt` ("Rules"). The decision, its
reasons and the open questions: [links.md](links.md). Check-specific decisions:

| Decision | Reason |
| --- | --- |
| Shared decision functions in `config/decide.rs` (`rule_allows`, `judge_link`, `covering_link`, `free`, `link_cycles`); `check`, the publish gate, the design editor and the viewer all call them | a rule re-implemented in a second place drifts |
| A measured edge is allowed only by a valid link (`deps/unlinked`; `deps/deny`, `deps/default`, `deps/independent` merged into it); the `rule` line says whether no link covers it or a covering link is denied | one fix per code: remove the edge or declare the link |
| Sibling code cycles are always errors (`deps.cycles` cut); cycles among links are information | code cycles are a defect; a drawn cycle is a plan to review |
| Exports are a plain list, checked after links (`deps/private` ignores rules and links) | a composite's internals stay private whatever is declared |
| Planned bricks (`design/planned`) and planned links (valid, no measured edge) are the todo of `check`, never findings (`deps/unused` cut) | unwritten code is planned work, not a violation |

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
- Id subjects (what follows `code\0`): `deps/unlinked|private` `from->to:kind:scope` by
  module; `deps/cycle` member names (prefixed by the level below root); `complexity/m` `file::fn::m`; `interface/unused`
  `leaf path:module:symbol`; `stale/file` the file; `stale/measure` `measure`; `bricks/unmapped`
  the module id.
- Text output is cargo-style. JSON is `{summary, diagnostics}`; the webview consumes it.
- `check --brick PATH` (repeatable) shows findings whose owner is the brick or a descendant,
  matched by level path. Owners: source brick of `deps/unlinked|private`, all members of
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
