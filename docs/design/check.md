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
  `config/member-outside-parent`. At the root an unmatched module is only `bricks/unmapped`;
  it takes part in no rule, cycle or complexity check, but its references still count as uses
  for `interface/unused`.
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
| Interfaces | Always evaluated; unused surface is advice (warn). See "Interfaces". |
| Links | `check::unused_links` lists every unbacked allow rule between two named bricks (backing as in the `via`/`kinds`/`scope` rules above; deny and selector rules excluded): every one is `deps/unused`, whether or not the ends have code (the agent hides it while coding with `--skip`). One list serves `deps/unused`, the design todo (kind `unused`) and the viewer. |
| Staleness | Candidates from `git diff` and `git status`, confirmed by sha256. Without git, hash everything. A dirty-tree cache also hashes everything. Config edits are never stale. |

`via` (optional on a rule): exported children of the rule's `to` composite. The rule matches only
an edge that enters `to` through one of them (child `p2[k+1]` of the lifted target path); a leaf
target never matches. `to` must be a composite brick and every name one of its `exports`, else
`config/unknown-via`. `deps/private` runs first, so exports stay the ceiling. Messages name the
entered child (`` `ui` uses `application` via `ports-out` ``).

Editing: `via` is a property of a rule, not part of its identity, and `rule set` keeps it unless
given. A `via` rule is backed only by a measured edge entering `to` through one of its
`via` children. Design ops keep `via` valid: `rename_brick` of a child rewrites the `via` of the
parent rules that target its composite; `remove_brick`, `move_brick`, unexporting via
`set_exports` and `ungroup` fail with `design/invalid` naming the rule when a `via` would dangle.

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
- Id subjects (what follows `code\0`): `deps/deny|default|private` `from->to:kind:scope` by
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
- `brickie.review.toml` (`[[accept]] id, reason`) accepts findings: counted, hidden in text, not
  failing the run. An agent never edits it.
- Exit 1: an unaccepted `error` exists. Exit 2: config, review, cache or argument error. Else 0.
  Warnings, accepted findings and findings hidden by `--brick` or `--skip` never change it.
  Hidden accepted findings are not counted in the footer.
- The cache is `.brickie/cache.json` beside the config; a missing, unparsable or wrong-schema
  cache exits 2. `stale/measure` is judged only when the cache records its measure settings.
  `stale/file`, `stale/measure` and `bricks/unmapped` have no owner.

## `brickie llms.txt`

Prints [../llms.txt](../llms.txt) (embedded, reads no project): the one guide for coding agents. It
holds the `measure -> check -> fix -> re-check` workflow, every diagnostic code with its fix,
the config schema and "Editing the design". An agent never edits rules or `brickie.review.toml` to
pass. The test `guide_documents_every_diagnostic_code` fails when a code is missing.

## Open items

- Config errors without a code: unknown tag, brick or kind, duplicate brick, include cycle, path
  cycle and unknown fields exit 2 but have no `config/` code in `codes.rs`. Proposal: give each a
  code so `llms.txt` can document its fix.
