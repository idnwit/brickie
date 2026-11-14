# Check

Scope: `brickie check`: config tree, member resolution, rules, interfaces, findings, `llms.txt`. Back to [../design.md](../design.md).

Example config: `docs/examples/brickie.toml`. Code: `crates/core/check`. `check` reads the cache
and configs and resolves the brick tree on every run. It needs no rust-analyzer. See [measure.md](measure.md).

## Config tree

- Each `brickie.toml` plus includes is one level. A brick with `path = "<dir>"` is a composite.
  Its level is `<dir>/brickie.toml`; `members` default to `path:<dir>/`.
- Names, tags and selectors are local to their file. `include` is recursive; a cycle is an error.
- Lists concatenate. `deps.default` and `deps.cycles` of the including file win.
- Unknown fields, duplicate bricks and unknown selectors are errors (exit 2).
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
- Below a composite, children must partition its modules: `config/unassigned`,
  `config/member-outside-parent`. At the root an unmatched module is only `bricks/unmapped`.

## Rules

| Rule | Semantics |
| --- | --- |
| Levels | An edge between leaves lifts to their lowest common ancestor `L`. Siblings under `L` are checked against `L`'s rules. Cost is O(depth) per edge. |
| Visibility (`deps/private`) | Each composite on the target path below `L` must export the next brick for the edge's scope. One error per module edge. Tests have no exemption. |
| Deps | Specificity brick 3, tag 2, `*` 1 per side; `via` adds 1 on the `to` side. Highest sum wins; deny wins a tie. No match: `deps.default`. Edges inside one brick or touching unmapped modules are ignored. |
| Cycles | Per level: SCCs over lifted prod edges among children, all kinds. One error per SCC. |
| Complexity | Limit is inclusive. Per function and metric the most specific rule applies; ancestor levels apply too, so the tightest limit wins and a nested file only tightens. Policy: [complexity.md](complexity.md). |
| Interfaces | Off in the Architecture preset: unused surface is advice. `interface/unused` (warn) per declared item with no outside use. |
| Links | `check::link_states` lists every unbacked allow rule between two named bricks (backing as in the `via`/`kinds`/`scope` rules above; deny and selector rules excluded): `planned` when an end or a descendant has no module, `unused` when both ends are fully measured. One list serves `deps/unused`, the design todo (`design/planned-edge` carries the state) and the viewer. |
| Staleness | Candidates from `git diff` and `git status`, confirmed by sha256. Without git, hash everything. A dirty-tree cache also hashes everything. Config edits are never stale. |

`via` (optional on a rule): exported children of the rule's `to` composite. The rule matches only
an edge that enters `to` through one of them (child `p2[k+1]` of the lifted target path); a leaf
target never matches. `to` must be a composite brick and every name one of its `exports`, else
`config/unknown-via`. `deps/private` runs first, so exports stay the ceiling. Messages name the
entered child (`` `ui` uses `application` via `ports-out` ``).

Editing: `via` is a property of a rule, not part of its identity, and `rule set` keeps it unless
given. A planned `via` rule is backed only by a measured edge entering `to` through one of its
`via` children. Design ops keep `via` valid: `rename_brick` of a child rewrites the `via` of the
parent rules that target its composite; `remove_brick`, `move_brick`, unexporting via
`set_exports` and `ungroup` fail with `design/invalid` naming the rule when a `via` would dangle.

## Interfaces

- A leaf declares the cached items that code outside the leaf can name. A composite declares the
  union of its exported children's interfaces.
- Uses come from reference edges (not `reexports`) from outside the brick, or from test code
  anywhere. A `#[test]` in a `mod tests` child counts; one in the item's own module does not.
- Not items: `macro_rules!` (no `pub` signature from rust-analyzer), a `pub fn` in a private
  module. A `pub(crate)` item shared between folder bricks is an item.
- Agents run `brickie interface <brick> [--json] [--from <brick>]`.

## Findings and exit codes

- A finding is a `Diagnostic` (`crates/core/check/src/diag/`). Its `id` is the first 8 hex of
  sha256 of `code\0subject`, so review decisions survive re-runs. Codes: `crates/core/model/src/codes.rs`.
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
- `brickie.review.toml` (`[[accept]] id, reason`) accepts findings: counted, hidden in text, not
  failing the run. An agent never edits it.
- Exit 1: an unaccepted `error` exists. Exit 2: config, review or cache error. Else 0.

## `brickie llms.txt`

Prints [../llms.txt](../llms.txt) (embedded, reads no project): the one guide for coding agents. It
holds the `measure -> check -> fix -> re-check` workflow, every diagnostic code with its fix,
the config schema and "Editing the design". An agent never edits rules or `brickie.review.toml` to
pass. The test `guide_documents_every_diagnostic_code` fails when a code is missing.
