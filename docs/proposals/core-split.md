# Proposal: split core, extract algorithm libraries

Status: **implemented** (2026-11-12). Deviations from the design below:

- The `tag:domain` complexity rule is `level = "warn"` for now (step 2 of "Path to zero
  complexity errors"), so CI stays green; switch it to `error` once the business functions are
  refactored. The previous root rule's `params = 6` limit was dropped (the spec lists none).
- `measure/scip` needs only package directories and target roots from cargo metadata; it
  declares its own input (`scip::layout::Layout`) and `run` converts the `workspace` metadata,
  so `scip` and `workspace` stay independent. `modules` (SCIP rows -> cache modules) lives in
  `scip/`; `files`, `edges` and `complexity` (metrics -> `FnTree`) live in `run/`.
- The crate root of `brickie-measure` / `brickie-check` belongs to the `run` / `check` brick
  (package member; the folder members are more specific and win).
- The `check` facade has thin wrapper functions (`dep_subject`, `complexity_limits`,
  `config_dir`, `set_review`; `DEPS_*` codes from `brickie-model`): a plain `pub use` would
  still resolve to the internal `rules` / `review` modules and be reported as `deps/private`.
- `brickie-metrics` has its own `FnTree` / `TreeNode` (no serde); `measure` converts them into
  the model types. `brickie-graph` has no LCA / tree-path helpers: the only tree walk
  (`rules::lift`) is config-specific and stays in `check`.
- `brickie-graph` gained two unit tests (SCC and cycle path); previously covered only through
  the rules tests.

## Problem

`brickie-core` is one crate of about 5.4k lines. Most of the functions over the complexity limits
are general-purpose algorithms, not brickie business logic:

| function | cognitive | nature |
| --- | --- | --- |
| `smart_route` | 131 | edge routing (graph drawing) |
| `measure_items`, `build`, `minimise`, `anneal_order`, `place_x` | 69, 45, 44, 24, 21 | Sugiyama layout |
| `walk`, `paths` | 69, 30 | cognitive / NPath over a syntax tree |
| `tokenize`, `use_items` | 55, 42 | Rust lexer, `use` parser |
| `find_cycle` | 16 | SCC / cycle path |

Complexity limits only tighten down the tree (the tightest limit of all ancestor levels wins).
Code that needs a looser limit therefore has to be a separate brick at the root level, not
inside `core`.

## Layout

```
crates/
  algo/        brickie.toml   exports [layout, graph, rustscan, metrics]   tag algo
    layout/    brickie-layout    graph layout and edge routing (from ui/src/layout), wasm-safe
    graph/     brickie-graph     SCC, cycle path, LCA / tree paths
    rustscan/  brickie-rustscan  Rust lexer + `use` item parser
    metrics/   brickie-metrics   cognitive, cyclomatic, NPath, nesting on tree-sitter
  core/        brickie.toml   exports [model, facts, measure, check]       tag domain
    model/     brickie-model     shared wasm-safe types (unchanged)
    facts/     brickie-facts     the cache (facts model, hashing, files)
    measure/   brickie-measure   brickie.toml: folder bricks scip/, workspace/, run (export: run)
    check/     brickie-check     brickie.toml: folder bricks diag ← config ← rules ← check, review
                                  (exports: check, diag, config; rules stay internal)
  web/         server, ui        tag domain (ui keeps components, state, model)
  cli/                           tag domain
```

- Crates only at real boundaries: `measure` and `check` are independent flows that share only
  `facts`, so Cargo also keeps `check` from pulling in tree-sitter or SCIP. Inside a crate, layers
  are folder bricks (`crate::module` members) checked by brickie: the tree mixes crates and
  folders and is three levels deep (root → core → measure/check).
- Within `core`: `facts → model`; `measure → facts, model`; `check → facts, model`; `measure` and
  `check` never use each other.
- Within `measure`: `run → scip, workspace`; `scip` and `workspace` are independent.
- Within `check`: `config → diag`, `rules → config, diag`, `check → rules, config, diag, review`.
  The web server's direct uses of `rules` (`dep_subject`, `complexity_limits`) go through the
  `check` facade.

## Algorithm libraries (`algo`)

- No dependency on any brickie crate (no `model`, no `facts`): std and external crates only
  (tree-sitter for `metrics`). Each has its own API; callers convert results (for example
  `measure` turns `metrics` output into `FnTree`).
- Root rules: `core → algo`, `web → algo` allowed; `algo → *` denied; `algo` exports every crate.
- Independent enough to be published or reused elsewhere later.
- Each library keeps its own unit tests; the move must not drop any.

## Complexity policy

```toml
# brickie.toml (root)
[[complexity]]            # business logic: strict, an error
bricks = "tag:domain"
cognitive = 15
nesting = 4
level = "error"

[[complexity]]            # algorithms: looser, a warning
bricks = "tag:algo"
cognitive = 60
nesting = 6
level = "warn"
```

Config as of 2026-11-12: `tag:domain` also limits `params` (6) and `lines` (80), still `warn`. A root
`bricks = "*"` ceiling (cognitive 60, nesting 6, lines 200) is an `error` for every brick, so it
also caps algorithms (`smart_route`, `walk`, `measure_items` fail until refactored or accepted). A
`scope = "test"` rule warns on extreme test functions (cognitive 40, lines 150).

Path to zero complexity errors:

1. Extract the algorithms into `algo`. Most heavy findings move there, where they are accepted as
   isolated, tested warnings.
2. Refactor the remaining business functions to the `error` limit step by step (mostly UI
   components, `normalise`, `test_files`). Until each step lands, the domain rule may start at
   `warn` so CI stays green.

## Brick descriptions

Every `[[brick]]` gets an optional one-line `desc`, written where the brick is declared, so the
map explains itself:

```toml
[[brick]]
name = "metrics"
members = ["brickie-metrics"]
desc = "Cognitive, cyclomatic, NPath and nesting of Rust functions (tree-sitter)"
```

- Config: `desc: Option<String>` on `Brick` (single line; no effect on checks).
- Viewer: the brick tooltip shows it under the name; the details panel shows it at the top
  for leaves, composites and faded neighbours. Graph data carries it on `BrickView`.
- `docs/llms.txt` documents the field; every brick in brickie's own configs gets a `desc`.

## Out of scope

- Publishing the `algo` crates.
- Refactoring business functions for complexity: a later step, one PR per area.
