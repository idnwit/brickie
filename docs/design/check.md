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
| Deps | Specificity brick 3, tag 2, `*` 1 per side. Highest sum wins; deny wins a tie. No match: `deps.default`. Edges inside one brick or touching unmapped modules are ignored. |
| Cycles | Per level: SCCs over lifted prod edges among children, all kinds. One error per SCC. |
| Complexity | Limit is inclusive. Per function and metric the most specific rule applies; ancestor levels apply too, so the tightest limit wins and a nested file only tightens. Policy: [complexity.md](complexity.md). |
| Interfaces | Off in the Architecture preset: unused surface is advice. `interface/unused` (warn) per declared item with no outside use. |
| Staleness | Candidates from `git diff` and `git status`, confirmed by sha256. Without git, hash everything. A dirty-tree cache also hashes everything. Config edits are never stale. |

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
- `brickie.review.toml` (`[[accept]] id, reason`) accepts findings: counted, hidden in text, not
  failing the run. An agent never edits it.
- Exit 1: an unaccepted `error` exists. Exit 2: config, review or cache error. Else 0.

## `brickie llms.txt`

Prints [../llms.txt](../llms.txt) (embedded, reads no project): the one guide for coding agents. It
holds the `measure -> check -> fix -> re-check` workflow, every diagnostic code with its fix,
the config schema and "Change sessions". An agent never edits rules or `brickie.review.toml` to
pass. The test `guide_documents_every_diagnostic_code` fails when a code is missing.
