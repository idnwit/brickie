# Levels

Scope: nested bricks, per-level checks, map-style zoom. Back to [../design.md](../design.md).

Map-style zoom over the brick tree. Inner parts may use only what the outer level allows. A
wrapping brick exposes something and outsiders may use only that. A brick without
`brickie.toml` is a leaf. A brick is a crate, a module or a folder of crates. Rules and
resolution: [check.md](check.md).

| Question | Decision | Reason |
| --- | --- | --- |
| Default `exports` | `[]` | an explicit facade |
| Composite discovery | `path = "<dir>"` with a required file | like a Rust module path; no scanning |
| Test edges | no exemption; write a scoped export `{ brick = "x", scope = "test" }` | exceptions are explicit |
| Modules left in a composite | partition error | their edges would escape every check |
| Re-exports | grant no access; move the item into an exported child | `exports` is not derived from `pub` |
| Brick mapping | not in the cache | editing config never needs a re-measure |
| Cycles | per composite | names are local to a level |

Rejected: `imports` lists, `@` selectors (lifting to the common ancestor gives the policy),
auto-discovery of nested configs, `pub`-derived exports, friend exports, cross-level selectors,
wheel-driven semantic zoom.

## Viewer

- The view shows one level at a time inside a frame. A private edge stops at the frame wall with
  a cross. The level lives in the query (`lv=`), so the server renders the same level.
- The server builds one `LevelView` per level. Edges are lifted per level and aggregated by
  (from, to, kind). Outside neighbours are faded rows. Code: `crates/web/server/src/data/graph/`.
- Outside edges (`bnd`): `children` draws each to the child it reaches; `frame` aggregates to
  the frame wall on the client. Scores and edge lists ignore frame edges.
