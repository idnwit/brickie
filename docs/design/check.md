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
- Lists concatenate. `deps.default` and `deps.cycles` of the including file win. Schema and
  defaults: `docs/llms.txt`.
- Unknown fields, duplicate bricks and unknown selectors are errors (exit 2).
- `[measure]` is root-only. `via` is validated after the composite levels load.
- Per-level semantics (exports, partition): [levels.md](levels.md).

## Resolution: member selectors

Each module gets one leaf path. At each level the most specific selector wins:

`path:` < package < `crate::module` prefix < `module:<id>`

- A longer path or module id beats a shorter one. Equal specificity across bricks is
  `config/overlap`.
- `module:<id>` is an **exact module selector**. It matches only that module, never its
  descendants and never other targets of the package.
- Partition errors stay errors under an exact selector. Exact membership is not an exclusion.
  Unassigned, overlapping and outside-parent selections are still reported.
- An unknown or inactive module ID stays unmatched.
- A `path:` selector is normalised lexically (no file system access) against the declaring
  file's directory. A member equal to a package name is a package before it is a module id.
  Equal members of one brick never conflict.
- Resolution descends: a module given to a composite is assigned again among its children, and
  the leaf path is where it stops.
- Below a composite, children must partition its modules: `config/unassigned`,
  `config/member-outside-parent`. At the root an unmatched module is `bricks/unmapped` (an error
  when the root `deps.default` is `deny`, since it would escape every rule; else a warning);
  it takes part in no rule, cycle or complexity check, but its references still count as uses
  for `interface/unused`.
- A composite owns the modules under its path. When a brick of an outer level wins a module that
  a member of the composite brick also matches (a more specific member), the child's member then
  reaches outside and the error names that brick, its level and the fix: make the module a child
  brick of the composite and export it if outside code uses it. Letting outer levels take such
  modules would bypass `exports`.
- `check` fails on the first partition error. Draft resolution (design preview) collects them as
  todo items instead: an overlap goes to the first brick, an unassigned module stays unmapped,
  an outside member is ignored.

## Rules

| Rule | Semantics |
| --- | --- |
| Levels | An edge between leaves lifts to their lowest common ancestor `L`. Siblings under `L` are checked against `L`'s rules. Cost is O(depth) per edge. |
| Visibility (`deps/private`) | Each composite on the target path below `L` must export the next brick for the edge's scope; the first one that does not gives the single error. Checked for every lifted edge: an `allow` rule never suppresses it and it never suppresses a rule finding. Exports are a ceiling; the source side is never checked. `pub use` grants nothing. Tests have no exemption. |
| Deps | Specificity brick 3, tag 2, `*` 1 per side; `via` adds 1 on the `to` side. Highest sum wins; deny wins a tie; of equal rules with one action the earlier in merged order wins. No match: `deps.default`. Edges inside one brick or touching unmapped modules are ignored. One finding per module edge (from, to, kind, scope). |
| Cycles | Per level: SCCs over lifted prod edges among children, all kinds. One error per SCC. |
| Complexity | Limit is inclusive. Per level the most specific rule that sets the metric applies (lower limit on a tie); the tightest limit across ancestor levels wins, so a nested file only tightens (on equal limits the outermost level owns the finding's rule). A function with no rule yields no finding. Policy: [complexity.md](complexity.md). |
| Independent | `independent = true` (root bricks only, leaf or composite; `config/independent-nested` otherwise; a composite counts as one lib, edges among its children follow its own level rules and its `exports` still apply) makes a brick act like an external library. Outbound: any edge to a non-independent brick or an unmapped workspace module is `deps/independent` regardless of rules; an edge to another independent brick is too, unless an explicit allow rule covers it (brick name or tag on both sides, never `*`; the normal rules then decide, so a deny still wins and cycles are still reported) (error; the only finding for that edge; not skippable or acceptable, a `--new` baseline may hold it); id subject like `deps/deny`. Inbound: edges into it are allowed by default (not `deps.default`); a normal rule pick, and only a winning deny whose `to` is a brick name or tag is a violation (`*` never denies it). An allow rule from an independent brick to a non-independent brick is `config/independent-rule`; `unused_links` skips allow rules into an independent brick unless the `from` is independent too. Complexity, interfaces and cycles are unchanged. |
| Interfaces | Always evaluated; unused surface is advice (warn). See "Interfaces". |
| Links | `check::unused_links` lists every unbacked allow rule between two named bricks (backing as in the `via`/`kinds`/`scope` rules above; deny and selector rules excluded): every one is `deps/unused`, whether or not the ends have code (the agent hides it while coding with `--skip`). One list serves `deps/unused`, the design todo (kind `unused`) and the viewer. |
| Staleness | Candidates from `git diff` and `git status`, confirmed by sha256. Without git, hash everything. A dirty-tree cache also hashes everything. A workspace member the cache does not know (root `Cargo.toml` `members`/`exclude`, `*` and `?` expanded on disk, other glob forms skipped) or a recorded package directory that lost its `Cargo.toml` is stale; a non-member crate is ignored. Config edits are never stale. A cache measured in another directory (copied or moved project) gives one `stale/measure` and no `stale/file`. `stale/*` is an error and cannot be skipped or accepted: a result on old facts is not a pass. |

`via` (optional on a rule): exported children of the rule's `to` composite. The rule matches only
an edge that enters `to` through one of them (child `p2[k+1]` of the lifted target path); a leaf
target never matches. `to` must be a composite brick and every name one of its `exports`, else
`config/unknown-via`. `deps/private` runs first, so exports stay the ceiling. Messages name the
entered child (`` `ui` uses `application` via `ports-out` ``).

Editing: `via` is a property of a rule, not part of its identity, and `rule set` keeps it unless
given. A `via` rule is backed only by a measured edge entering `to` through one of its
`via` children. Design ops keep `via` valid: `rename_brick` of a child rewrites the `via` of the
parent rules that target its composite; `remove_brick`, `move_brick`, unexporting via
`remove_export` and `ungroup` fail with `design/invalid` naming the rule when a `via` would dangle.

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
- Id subjects (what follows `code\0`): `deps/deny|default|private|independent` `from->to:kind:scope` by
  module; `deps/cycle` member names (prefixed by the level below root); `deps/unused`
  `level:from->to:kinds:scope`; `complexity/m` `file::fn::m`; `interface/unused`
  `leaf path:module:symbol`; `stale/file` the file; `stale/measure` `measure`; `bricks/unmapped`
  the module id.
- Text output is cargo-style. JSON is `{summary, diagnostics}`; the webview consumes it.
- `deps/unused` (warn): id from level, ends, kinds and scope (no file line), so accepting it survives
  toml edits.
- `check --brick PATH` (repeatable) shows findings whose owner is the brick or a descendant,
  matched by level path. Owners: source brick of `deps/deny|default|private`, all members of
  `deps/cycle`, both ends of `deps/unused`, the defining brick of `interface/*`, the function's
  brick of `complexity/*`; ownerless findings (`stale/file`, `bricks/unmapped`) always show.
  Everything is evaluated and `[[accept]]` applied first; the exit code uses the shown set; the
  footer is `outside scope: N errors, M warnings`.
- `check --skip CODE` (repeatable or comma separated) hides only findings whose level is warning
  (an error finding with a skipped code still shows and counts); footer `skipped: N warnings
  (codes)`. JSON adds `outside_scope` and `skipped` when used. Unknown codes and bricks exit 2.
- `brickie.review.toml` (`[[accept]] id, reason, weight?`) accepts findings: counted, hidden in
  text, not failing the run. An agent never edits it. With `weight` the acceptance holds while the
  finding's current weight (`Finding::weight`) is <= it; above it the finding is open at its own
  level with the note `worse than accepted: N -> M`. The viewer's Accept always writes the current
  weight; a hand-written entry without `weight` accepts at any size. An accept whose id matches no
  finding gives the ownerless warning `review/orphan` (span: the entry in the review file),
  skipped while any `stale/*` finding exists.
- Exit 1: an unaccepted `error` exists. Exit 2: config, review, cache, baseline or argument error. Else 0.
  Warnings, accepted findings and findings hidden by `--brick` or `--skip` never change it.
  Hidden accepted findings are not counted in the footer.
- The cache is `.brickie/cache.json` beside the config; a missing, unparsable or wrong-schema
  cache exits 2. `stale/measure` is judged only when the cache records its measure settings.
  `stale/file`, `stale/measure` and `bricks/unmapped` have no owner.

## Gate for coding agents

Decision (two independent reviews, then tests on a fixture and on this repo): `check` must be
usable as the only signal of a hook or CI step, so exit 0 means "checked and not worse".

| Hole found | Decision |
| --- | --- |
| Code edited after `measure` passed with a warning | `stale/*` is an error; `check --measure` re-measures when the cache is missing or stale |
| A new crate or module in no brick passed under `default = "deny"` | `bricks/unmapped` is an error under a root default deny (no new key: strictness is not opted into twice) |
| `cfg(any(test, ..))`, `feature = "test.."` and a prod folder `src/tests/` counted as test scope | scope is test only for `cfg(test)`, `test` under `all(..)`, and Cargo's `tests/` and `benches/` ([measure.md](measure.md)) |
| A codebase with old findings gave no usable exit code | `check --save-baseline` and `check --new` |
| The agent can relax a rule or accept a finding | `baseline/rules-changed` under `--new`; the finding help no longer prints a ready `rule add` command |
| Text output costs about 6 lines per finding | `--format short` |
| A project directory copied with its `.brickie/` was checked against the old location | the cache records where it was measured; another location is `stale/measure` |
| A new workspace crate was not stale, so `--measure` did not re-measure and the crate went unchecked | workspace members unknown to the cache are `stale/file` |
| Old facts that break the config (`config/unassigned` for a removed crate) blocked `--measure` | `--measure` judges staleness from the cache alone, before the config loads |

Baseline (`.brickie/baseline.json`, code in `check/baseline.rs`):

- Holds the commit, the sha256 of every loaded `brickie.toml` (includes and composites) and of
  `brickie.review.toml`, and per unaccepted finding its id, code, message and weight. `stale/*`
  is never recorded.
- Weight: the metric value for `complexity/*`, the number of reference sites for
  `deps/deny|default|private`, else 1. `--new` hides a recorded finding only while its weight
  has not grown, so an old finding cannot get worse unseen.
- The baseline filter runs before `--brick` and `--skip`. `baseline/rules-changed` has no owner
  and cannot be accepted.

Limits, open:

- Nothing stops an agent from running `--save-baseline` or editing `.brickie/`; the guide forbids
  it. A CI job that builds the baseline from the base branch is the hard gate.
- Ids follow file and module names: moving code makes a finding new and orphans its accept
  (`review/orphan` warns).
- References to external crates are not edges, so "domain must not use an HTTP client" cannot be
  written. Proposal: bricks for external crates.
- `check --brick` exits 0 with errors outside the scope (by design, for mid-work); a gate runs
  the full check.
- `interface/unused` cannot be fixed for a `pub(crate)` item of a crate root module that other
  modules of the same brick use: it is the narrowest visibility that compiles, yet sibling folder
  bricks can name it, so it counts as interface (three such warnings remain on this repo). A use
  from outside Rust (a `#[wasm_bindgen]` export called from JavaScript) is not seen either.
  Proposal: count a use from the same brick when the item is not `pub`.

## `brickie llms.txt`

Prints [../llms.txt](../llms.txt) (embedded, reads no project): the one guide for coding agents. It
holds the `measure -> check -> fix -> re-check` workflow, every diagnostic code with its fix,
the config schema and "Editing the design". An agent never edits rules or `brickie.review.toml` to
pass. The test `guide_documents_every_diagnostic_code` fails when a code is missing.

## Open items

- Config errors without a code: unknown tag, brick or kind, duplicate brick, include cycle, path
  cycle and unknown fields exit 2 but have no `config/` code in `codes.rs`. Proposal: give each a
  code so `llms.txt` can document its fix.
