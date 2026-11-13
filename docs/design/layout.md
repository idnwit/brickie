# Code layout

Scope: crates, folders and crate boundaries of this repo. Back to [../design.md](../design.md).

Sixteen crates in layers under `crates/`. brickie checks itself with the root `brickie.toml` and
nested files of the folder bricks. Layering: `cli -> web -> core -> lib`, `web -> lib`. `lib`
depends on no brickie crate, and its parts (`algo`, `codeview`) never use each other. Folders by
concern, files about 300 lines or fewer (tests aside), no flat `src/`.

| path | contents |
| --- | --- |
| `crates/lib/algo/{layout,graph,rustscan,metrics}` | pure algorithms: layout and routing, SCC, Rust tokenizer, complexity metrics |
| `crates/lib/codeview/{port,app,host}` | the code viewer, independent of brickie |
| `crates/core/model` | light wasm-safe types, every diagnostic code, change types |
| `crates/core/facts` | the cache schema and `cache.lock` |
| `crates/core/measure` | `workspace/`, `scip/`, `run/`: code to cache |
| `crates/core/change` | change sessions: store, edit, preview, publish |
| `crates/core/check` | config, rules, interfaces, review, diagnostics (shared by check and serve) |
| `crates/web/{api,server,ui}` | wasm-safe port, axum server, hydrated sycamore UI |
| `crates/cli` | the binary and integration tests |

## Crate boundaries

- Crates exist only at real boundaries, so Cargo enforces what brickie checks. Inside a crate,
  layers are folder bricks (`crate::module` members).
- Limits only tighten down the tree. Code needing a looser limit is its own root-level brick,
  not part of `core`. That is why `algo` is separate.
- `algo` (tag `algo`): std and external crates only, no brickie crate, own API and unit tests.
  Callers convert results. `core -> algo` and `web -> algo` are allowed, `algo -> *` denied.
- `core` (tag `domain`): `measure -> facts, model`; `check -> facts, model`. `measure` and
  `check` never use each other and share only `facts`, so `check` pulls in neither tree-sitter
  nor SCIP. Inside `measure`: `run -> scip, workspace`, independent of each other.
- A crate root belongs to the `run` / `check` brick (a package member; folder members win).
  The `check` facade uses thin wrapper functions, because a plain `pub use` still resolves to the
  internal module and is `deps/private`.
- `[[brick]] desc` is a one-line label with no effect on checks.
- Complexity limits per tag: [complexity.md](complexity.md).
