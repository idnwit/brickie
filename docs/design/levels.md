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
| Test edges | no exemption; exports are plain names for every scope | one way to open a composite |
| Modules left in a composite | partition error | their edges would escape every check |
| Outer brick taking a module from inside a composite | `config/member-outside-parent`, naming the outer brick | it would bypass `exports`; fix: make it a child and export it |
| Re-exports | grant no access; move the item into an exported child | `exports` is not derived from `pub` |
| Brick mapping | not in the cache | editing config never needs a re-measure |
| `independent` brick flag | root level only (leaf or composite; a composite is one lib, its children use each other under its own rules and `exports` still apply) | lifting an edge to the common ancestor hides the flag below the root |
| Cycles | per composite | names are local to a level |

Rejected: `imports` lists, `@` selectors (lifting to the common ancestor gives the policy),
auto-discovery of nested configs, `pub`-derived exports, friend exports, cross-level selectors,
wheel-driven semantic zoom.

## Viewer

- The view shows one level at a time inside a frame. A private edge stops at the frame wall (no
  cross glyph); its tooltip says "private: stops at the wall". The level lives in the query (`lv=`), so the server renders the same level.
- The server builds one `LevelView` per level. Edges are lifted per level and aggregated by
  (from, to, kind). Outside neighbours are faded rows. Code: `crates/web/server/src/data/graph/`.
- Outside edges (`bnd`): `children` draws each to the child it reaches; `frame` aggregates to
  the frame wall on the client. Scores and edge lists ignore frame edges.
- Independent libs (`independent = true`) sit in their tag row like any brick, marked by the text
  "independent lib", a dashed box and a tooltip. Server-side (`ModEdge.hidden`), a clean edge into
  one from a non-independent brick is not served: no edge, no ghost, no fan-in or score effect.
  Edges carrying any finding and all outbound edges are drawn. The
  inspector says "used by N more brick(s); edges hidden: independent lib" (`BrickView.hidden_in`),
  so a missing line never reads as "no dependency".
