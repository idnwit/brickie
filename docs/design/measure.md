# Measure

Scope: what `brickie measure` reads, the facts it records, the cache layout. Back to [../design.md](../design.md).

`measure` is the only step that reads code. Sources: `cargo metadata`, rust-analyzer SCIP (edges),
tree-sitter-rust (complexity). The cache holds facts and a content hash per file. It holds no
brick data.

- Every measure holds `.brickie/cache.lock` (holder PID inside) and writes the cache through a
  temp file and rename. A second measure fails at once: a wait looks hung. A dead holder's lock
  is taken over.
- Stale cache: changed, added or deleted `.rs` files give `stale/file` warnings (see [check.md](check.md)).

## Features and targets

Proposal and phases: [../proposals/features.md](../proposals/features.md).

| Decision | Reason |
| --- | --- |
| Always all cargo features (`cargo.features = "all"` in a rust-analyzer `--config-path` JSON); no setting | the design covers every code path; additive features (P1) make it buildable |
| One `scip` run for the host, one per `[measure] additional_targets` entry (`cargo.target`); per-file edges and items are unioned and deduplicated | rule checks see every edge; a file compiled by any run counts as compiled |
| The config JSON uses the nested form `{"cargo": {...}}`; flat `cargo.features` keys are silently ignored by rust-analyzer | verified on rust-analyzer's `scip` command |
| A failed run reports its target and says exclusive features must be made additive | P1 enforced by construction |
| `measure` prints one `hint:` line per `cfg(target_arch/os = ...)` family found by a text scan that neither the host (`rustc -vV`) nor an additional target covers | never add a target silently; its cost stays visible |
| The cache records `measured { features, targets }`; `check` warns `stale/measure` when the config differs | the settings are part of the cache key; a cache without them is not judged |

## Facts

- Units: crate, module (`crate::a::b`), file. A module maps to at most one leaf brick.
- **`to` of an edge is the module that defines the item** (SCIP). A re-export is not an edge
  target. There is no path-based re-attribution.
- Each reference site gives one edge kind:

  | kind | sites |
  | --- | --- |
  | `uses` | references outside `use` items, plus leaves of private `use` |
  | `reexports` | leaves of a `use` with any `pub` visibility |
  | `implements` | trait path of `impl Trait for X` in another module (rust-analyzer emits no SCIP relationships, so syntax + SCIP) |

- Evidence `written` is the path as written at the site, never expanded through imports.
- Edge `scope` is `test` for `#[cfg(test)]`, `tests/`, `benches/`, else `prod`.
- Function metrics: `cognitive`, `cyclomatic`, `nesting`, `paths` (NPath), `params` (no `self`),
  `lines`, `scope`. Macro bodies are not parsed, so branches inside `println!`, `vec![]` and
  `view!` are invisible. Scoring follows SonarSource.

## Cache layout (schema_version 6)

`.brickie/cache.json` sits next to the config. Top level: `schema_version`, `tool_version`,
`rust_analyzer`, `measured`, `workspace_root`, `git`, `crates`, `modules`, `files`.

- Ids: a library crate is its package name, other targets `pkg/kind/target`, a module
  `<crate id>::a::b`. A crate records `dir` (package dir) for `path:` members.
- `files` is keyed by path: hash, size, lines, scope, module, `functions`, outgoing `edges` with
  evidence. There is no aggregated edge list: `Cache::edges()` derives module edges at read
  time, so a per-file refresh stays possible.
- Every `.rs` file under a crate root is recorded. One the compiler never saw has
  `compiled: false`, so staleness can tell it from an added file.
- `items` per file: every non-test `pub` or `pub(crate)` item (`symbol`, `module`, `kind`, `name`,
  one-line `signature`, `doc`, `line`, `vis`, `visible_in`, `exported`). Trait members, trait-impl
  items, fields, variants and modules are not items. Only declared items are cached.
- Unreadable source: `measure` warns once per path and records an empty row (`lines = 0`).
