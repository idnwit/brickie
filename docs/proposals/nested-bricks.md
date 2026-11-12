# Proposal: nested bricks (onion layers, map-style zoom)

Status: **agreed design**, reconciled from two independent reviews (architecture,
implementation) and the user's decisions (2026-11-12). Phases 1-4 **implemented** (see
`docs/design.md`, "Check" and "View"); the viewer's deviations from the View section below are
listed under "Levels" in `docs/design.md`.

Implementation notes: the finding's level is the JSON field `brick_level` (`level` already holds
the severity). `deps/private` is reported at the level of the composite that hides the child,
with that child in `bricks`; a complexity finding at the level its leaf is a child of.

## Idea

A brick is a black box at its parent's level. A brick with internal structure (a **composite**)
has its own `brickie.toml` describing its child bricks; a brick without one is a **leaf**. Each
level is verified on its own and the viewer zooms level by level, like a map.

User requirements (fixed):

1. Inner parts may only use what the outer level allows.
2. A wrapping brick exposes something; outsiders may only use what it exposes.
3. A brick without `brickie.toml` is a leaf.
4. A brick can be a crate, a module, or a folder of several crates.

## Model

### Tree

- A brick is a composite when it declares `path = "<dir>"` (relative to the declaring file),
  like a Rust `mod` path: its config is `<dir>/brickie.toml`, and a missing file is an error
  (`config/missing`, exit 2). Its members default to `path:<dir>/`. A `brickie.toml` in a
  directory no brick points at is never read.
- A composite's file holds its children (`[[brick]]`), `[deps]`, `[[deps.rule]]`,
  `[[complexity]]`, `include`, and its facade `exports`.
- Children **partition** the composite's modules exactly: a child member outside the parent is
  `config/member-outside-parent`, a module in two children `config/overlap`, a module in no child
  `config/unassigned` (all exit 2). A composite owns no modules directly, so any two distinct
  leaves always have two distinct siblings under their lowest common ancestor.
- Names and tags are local to a file; rules only select that file's own children. No selectors
  across levels.
- Members: `crate`, `crate::module`, or `path:<dir>/` (crates whose manifest is under the folder).
  The most specific match wins: `path:` < package < module; a longer `path:` beats a shorter one.

### Contract

- **Policy (req. 1)** lives in the parent: its `[deps]` rules decide which children may depend on
  which. There is no `imports` list: every edge leaving a composite is lifted and checked
  against each enclosing level's rules, so an inner part can never use more than its parent may.
- **Facade (req. 2)** lives in the composite itself: `exports = ["api"]` lists direct children
  visible from outside. Absent `exports` means `[]` (nothing visible). Siblings inside a
  composite always see each other. Re-exporting through the facade does not grant access, since
  an edge points at the defining module: to expose an item, move it into an exported child.
  `exports` is not derived from Rust `pub`.
- **No built-in exceptions**: test edges are checked like prod edges. An exception is an explicit
  rule, e.g. `exports = ["api", { brick = "internal", scope = "test" }]` exposes `internal` to
  test-scope edges only.

### Example

```toml
# brickie.toml (root)
[[brick]]
name = "web"
path = "crates/web"               # composite: reads crates/web/brickie.toml
tags = ["app"]

[[brick]]
name = "core"
members = ["acme-core"]
tags = ["core"]

[deps]
default = "deny"
cycles = "deny"

[[deps.rule]]
from = "tag:app"
to = "tag:core"
action = "allow"
```

```toml
# crates/web/brickie.toml
exports = ["api"]

[[brick]]
name = "api"
members = ["web-api"]

[[brick]]
name = "ui"
members = ["web-ui"]

[[brick]]
name = "shared"
members = ["web-shared"]

[deps]
default = "deny"

[[deps.rule]]
from = "*"
to = "shared"
action = "allow"

[[deps.rule]]
from = "ui"
to = "api"
action = "allow"

[[complexity]]
cognitive = 12
```

### Check algorithm

Resolve every module to a leaf path (`["web", "api"]`). For each module edge `m1 -> m2`:

1. Equal paths: the edge is internal to a leaf and is ignored. Unmapped: existing handling.
2. `k` = first index where the paths differ; `L` = the composite at `path[..k]`; `X`, `Y` = the
   siblings at index `k`.
3. **Policy**: `X -> Y` against `L`'s rules (existing specificity) gives `deps/deny` or
   `deps/default`.
4. **Visibility**: for each `j > k` on `m2`'s path, `path2[j]` must be in the `exports` of the
   composite `path2[..j]`; the first failure gives `deps/private`. A scoped export entry counts only for edges of
   that scope.
5. **Cycles**: per composite, SCCs over lifted prod edges among its children (`deps/cycle`).
6. **Complexity**: the rules of every ancestor level apply to the leaves below it; the tightest
   limit per metric wins, so a child file can only tighten.

Each finding carries `level` (`root/web`) and the config file that owns the rule, so an agent
knows which file to edit. The cost is O(depth) per edge.

### Cache and measure

The brick mapping moves out of the cache. `measure` stores only crates, modules, files, edges and
functions. `check` and `view` resolve the tree from the configs, which is a cheap, pure step.
Editing any `brickie.toml` therefore never needs a re-measure, and the config-hash staleness
check (`stale/bricks`) goes away.

### View

Prototype agreed with the user (2026-11-12). The server emits the tree (nodes with `path`,
`has_children`, `exports`) and aggregated edges per level (`{level, from, to, count, edge ids,
worst_level}`); the UI is a function of the current level.

- **One level at a time**: the canvas lays out the children of the current composite inside a
  frame labelled with its name, `exports` and config file. Composites have a double border, a
  badge with the number of findings in their subtree and their exported children drawn as ports
  on the border; leaves look like today's nodes.
- **Navigation**: double-click on a composite, or a click on its zoom icon (small magnifier with
  a plus at the start of its tag line), zooms in (short scale
  transition, off under `prefers-reduced-motion`); Backspace, Esc, the back button or the
  breadcrumb (`root › web`) zoom out. The breadcrumb is an overlay at the top-left corner of
  the canvas, fixed while the graph pans and zooms. The level is kept in the URL hash, so a link opens the same level.
- **Neighbours**: bricks outside the current level that have edges crossing its boundary are
  drawn faded on the left (sources) and right (targets), lifted to the sibling of the nearest
  enclosing level.
- **Edges**: one aggregated edge per brick pair; width grows with the number of references,
  red when any underlying edge has a finding. A `deps/private` edge reported at this level from
  outside stops at the frame wall with a cross. Clicking an edge lists the module edges, their
  evidence (opens the source popup) and their findings.
- **Peek**: hovering a composite shows its children, marking the exported ones.
- **Findings panel**: all findings, with a filter "this level and below"; clicking one jumps to
  the finding's `level` and highlights its edge.
- **Leaves**: selecting a leaf shows its modules and functions with complexity, as today.

Out of v1: wheel-driven semantic zoom, and opening a composite inline inside its parent level.

## Diagnostics

The existing `deps/deny`, `deps/default` and `deps/cycle` codes gain a `level`. New codes:
`deps/private`, `config/member-outside-parent`, `config/overlap`, `config/unassigned`,
`config/unknown-export`, `config/missing`. Each needs an entry in `codes.rs` and `docs/llms.txt`.

## Phases

1. Move the brick mapping out of the cache.
2. Tree loading (`path`, `path:`, partition checks); per-level deps and cycles via lifting.
3. `exports` and `deps/private`; inherited complexity.
4. Viewer zoom.

## Out of v1

- `imports` lists.
- Auto-discovery of nested configs.
- Item-level or `pub`-derived exports.
- Consumer-specific ("friend") exports.
- Cross-level selectors.
- Opening one composite inline inside another level's view.

## Review record

Both reviewers agreed on the following:

- Drop `imports` / `@`, since lifting to the lowest common ancestor already gives req. 1.
- `exports` is owned by the composite.
- Names are local to a level.
- Per-level cycles.

Where they differed, this proposal resolves as follows:

- **Default `exports`**: `[]` (architecture) rather than "all" (implementation), because req. 2
  asks for an explicit facade.
- **Composite discovery**: the user chose `path = "<dir>"` with a required
  `<dir>/brickie.toml` (like a Rust module path), instead of explicit `config =` or
  directory scanning.
- **Test edges**: the user chose no built-in exemption; exceptions are written as rules
  (scoped export entries).
- **Modules left in a composite**: a partition error (architecture) rather than an unmapped
  warning. Otherwise their edges would escape every check.
- **Complexity**: all ancestor rules apply (implementation) rather than validating that a child
  only tightens. The effect is the same and there is no extra validation.
- **Brick mapping**: out of the cache (implementation).
