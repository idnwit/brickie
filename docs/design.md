# brickie: design

Status: steps 1-4 implemented (`measure`, `check`, `serve`, `llms.txt`, cleaned and restructured
code); the rest is agreed direction (2026-11-11). The previous prototype (Brickwork PoC, Python
viewer, experimental metrics) is not part of the public history.

## Goal

One Rust binary, `brickie`, with two users:

1. **CI**: measure a workspace, apply the project's own rules, exit non-zero on violations.
2. **Reviewer webview**: see bricks, dependency edges, violations and per-function complexity next
   to the source, to review what coding agents changed.

The tool has **no opinion about architecture**: no built-in layers, families or zones. Rules are
written by the user in `brickie.toml`; a preset is just a rules file other configs can include.

## Two parts: measure, then rules

```
brickie measure ──► .brickie/cache.json ──► brickie check  ──► findings (text/json, exit code)
     (no config needed)                 └──► brickie serve  ──► webview   (both read brickie.toml)
```

- `measure` is the only step that reads code. It writes a cache with facts (crates, modules,
  files, edges, functions) and a content hash per source file; no brick data.
- `check` and `serve` read the cache and the configs and resolve the brick tree on every run, so
  they are fast and need no rust-analyzer. Changing any `brickie.toml` (bricks or rules) never
  requires re-measuring.
- **Staleness**: `check`/`serve` re-hash the files listed in the cache; changed, added or deleted
  `.rs` files are reported as `warn` findings ("measured before this change; re-run measure").

### Change detection and incremental measure

The cache stores the git `HEAD` commit at measure time, a dirty flag, and per-file data keyed by
path (sha256, size, functions, outgoing edges with evidence).

- **Detect changes**: candidates come from git (`git diff <cached commit>` plus `git status` for
  modified/untracked files); candidates are confirmed by sha256, since content may be unchanged.
  Without git, or when the cached commit is gone, hash every file.
- **Complexity is per file** (tree-sitter only, no SCIP): changed files are re-measured
  immediately, so complexity findings are always current.
- **Dependencies need SCIP for the whole workspace**: incoming edges into a changed file depend on
  unchanged files. Changed files are marked "deps stale". If the syntax diff touches `use`,
  `impl` or `pub` signatures, check reports that a full re-measure is needed; body-only edits
  keep the cached edges. The full SCIP run happens in CI or on demand.
- **PR mode** (later step): report only findings that are new or located in files changed
  against a base branch, so reviewers see what the agent just introduced.

### 1. Measure: `brickie measure [--config brickie.toml] [--scip index.scip]`

Sources: `cargo metadata`, rust-analyzer SCIP (dependencies), tree-sitter-rust (complexity).

- **Units**: crate, module (`crate::a::b`), file; `check` maps each module to at most one leaf
  brick.
- **Dependency edges** between modules, each with `kind`, `scope` and evidence (file:line, symbol,
  `written`). **`to` is always the module that defines the referenced item**, as resolved by SCIP.
  A reference through a re-export (`use acme_engine::AudioEngine` where `AudioEngine` is defined in
  `acme_engine::audio_engine` and re-exported at the crate root) is an edge to `audio_engine`, not
  to the crate root; there is no path-based re-attribution. The evidence keeps the path as written,
  so the two can be read side by side. Each reference site yields exactly one kind:

  | kind | sites | source |
  | --- | --- | --- |
  | `uses` | any reference outside `use` items (call, type, constant, method), plus the leaves of private `use` imports (no visibility keyword) | SCIP reference |
  | `reexports` | the leaves of a `use` with any `pub` visibility (`pub`, `pub(crate)`, `pub(super)`, `pub(in ..)`) | SCIP + syntax |
  | `implements` | the trait path of `impl Trait for X` where the trait lives in another module (not also `uses`) | syntax (`impl Trait for X`) + SCIP |

  Evidence `written` is the path as written at the site, unresolved: for a `use` leaf the full path
  through its groups (`use a::{b::C}` gives `a::b::C`); for a body reference the identifier with
  the `a::b::` qualifiers directly before it (`acme_engine::AudioEngine`), or the plain name
  (`double`) when written plainly, never expanded through imports. `check` notes and the viewer
  show `written X -> defined in <module>`.

  `scope` is `test` for `#[cfg(test)]`, `cfg(any(test, ..))`, `tests/`, `benches/`, otherwise `prod`.
- **Functions**: `cognitive`, `cyclomatic`, `nesting`, `paths` (NPath: sequence adds, nesting
  multiplies), `params`, `lines`, `scope`.

### 2. Rules: `brickie check [--config brickie.toml] [--format text|json]`

```toml
include = ["presets/layered.toml"]   # a preset is a file with the same schema, merged in

[[brick]]                            # group modules into a brick, tag it
name = "engine"
members = ["my-engine"]              # package, crate::module prefix, module:<exact-id> or path:<dir>/
tags = ["core"]

[[brick]]
name = "web"
path = "crates/web"                  # composite: its children are in crates/web/brickie.toml

[deps]
default = "deny"                     # edges matching no rule
cycles = "deny"                      # cycles between bricks

[[deps.rule]]
from = "tag:app"                     # brick name | tag:<tag> | "*"
to = "tag:core"
kinds = ["uses", "implements"]       # default: all kinds
scope = "prod"                       # default: prod
action = "allow"                     # allow | deny
reason = "app calls into core"

[[complexity]]
bricks = "tag:core"
cognitive = 15
nesting = 3
paths = 200
params = 5
level = "error"                      # error (fails CI) | warn
```

- The most specific matching rule wins (brick > tag > `*`); on a tie, deny wins.
- Each finding has a stable id (hash of rule + subject) so review decisions survive re-runs.
- Exit code 1 when an `error` finding is not accepted in `brickie.review.toml`.

### 3. Serve: `brickie serve [--config brickie.toml] [--port 7881] [--bind 127.0.0.1]`

Implemented; details in "Serve (implemented)" below. (Named `view` before; renamed when the
server gained the measure job, no alias.)

## Removed from the prototype

The Python prototype viewer, SQL rule files, Brickwork families/presets, the old contract/policy/render
code, the SQL graph server for agents (`serve`/`query`), capabilities, and experimental metrics
(S10–S15, N*, duplication, magic numbers, git hotspots).

## Distribution

- Public repo `idnwit/brickie`, fresh history; protected default branch, changes via PR with CI.
- CI: fmt, clippy `-D warnings`, test.
- Every push to the default branch builds Linux x86-64 and macOS binaries into a rolling `dev`
  release.
- `release-plz` for versioned releases.
- `install.sh` (curl | sh) installs `dev` or a given version.

## Steps

1. `measure` → cache (deps + complexity + file hashes), tests on a fixture. **Implemented** (see
   "Cache layout" below).
2. `check`: **Implemented** (see "Check" below).
3. `serve`: **Implemented** (see "Serve (implemented)"). Shows the same `Diagnostic` values as
   `check`; every file:line span opens the code viewer (in a popup) at that line.
   The UX is a port of the Python prototype viewer (JS in one HTML file, not published): client-side Sugiyama layout, presets and data layers, selection
   scoping, complexity tree (now as gutter bands in the code viewer). Ported and dropped parts are listed in
   "Port of the prototype viewer".
4. **Implemented.** Old graph-server/contract stack, SQL rules, presets, Python prototype and
   obsolete docs removed; `src/` restructured into folders of small files (see "Code layout");
   English `README.md`; `brickie llms.txt` prints `docs/llms.txt`, a static guide for coding
   agents embedded with `include_str!` (workflow `measure` -> `check` -> fix -> re-check, every
   diagnostic code and how to fix it, the `brickie.toml` schema, and the constraints: an agent
   never edits rules or `brickie.review.toml` to pass; accepting a finding is a human reviewer
   decision). A test requires the guide to mention every code in `crates/core/model/src/codes.rs`.
5. Publish: repo, CI, branch protection, dev release, install script, release-plz.
6. Incremental: re-measure complexity of changed files, deps-stale detection, PR mode.
7. Nested bricks (onion levels, per-level checks, `exports` facades, map-style zoom in the
   viewer). Spec and phases in [proposals/nested-bricks.md](proposals/nested-bricks.md).
   Phases 1-3 **implemented**: brick mapping out of the cache (then schema 5), config tree with
   `path`/`path:`, partition checks, per-level deps and cycles via lifting, `exports` /
   `deps/private`, inherited complexity (see "Check"). Phase 4 **implemented**: the viewer shows one
   level at a time with map-style zoom (see "View", "Levels").
8. Brick interfaces (**implemented**, [proposals/brick-interface.md](proposals/brick-interface.md)):
   items in the cache (schema 6), `interface/unused`, `brickie interface <brick> [--json]
   [--from <brick>]`, the viewer's Interface section and the code behind a selected edge
   (`POST /api/interface`, `POST /api/edge-uses`).
9. Design editor and change sessions ([proposals/editor.md](proposals/editor.md)):
   a draw.io-like edit mode of the viewer; humans and one coding agent record ops in a single
   open change (never editing `brickie.toml` directly), reviewed per op and published with
   scaffolding; `brickie change` tells the agent what changed, by whom, and what is left.
   Phases 1, 2 and 6 **implemented**: `crates/core/change` (op log in `.brickie/change/`,
   replay with conflicts, preview as a `FileSource` for the loader, review, semantic diff, todo,
   publish with refusal checks and scaffolding, `move_brick`, brick kinds, link options, layout
   hints), `brickie check --change`, `brickie change` (open, show, `--since`, `--todo`,
   propose, accept, reject, publish `--dry-run`, discard), `change/*` codes and the llms.txt
   "Change sessions" section. Phases 3 and 4 (the viewer's edit mode) **implemented**: change
   endpoints behind a loopback/same-origin write guard, the preview shown with `change=1`,
   floating toolbar, typed ports, inspector, move by drag, ops with review, publish dialog,
   then the editor UX spec (`docs/proposals/editor-ux.md`: design panel tabs incl. Rules,
   complexity, exports, includes and presets; in-place patching) (see "View", "Design
   editor"). MCP (7) is open.

## Code layout (implemented)

A virtual workspace of sixteen crates, grouped by layer (brickie checks this with the root
`brickie.toml` and the nested `brickie.toml` of `crates/lib`, `crates/lib/algo`,
`crates/lib/codeview`, `crates/core`, `crates/core/measure`, `crates/core/check`, `crates/web`,
`crates/web/server` and `crates/web/ui` (and the leaf `crates/web/api`): cli -> web -> core ->
lib, web -> lib; `lib` (algo, codeview) depends on no brickie crate and its two parts never use
each other; see
`docs/proposals/core-split.md` and `docs/proposals/code-viewer.md`):

```
crates/lib/algo/           general-purpose libraries, no brickie types (tag algo, looser limits)
  layout/              brickie-layout: Sugiyama layout and edge routing (wasm-safe)
  graph/               brickie-graph: SCC (Tarjan), cycle path
  rustscan/            brickie-rustscan: Rust tokenizer, `use` item scanner
  metrics/             brickie-metrics: cognitive, cyclomatic, NPath, nesting, decision tree
                       (tree-sitter)
crates/core/model/     brickie-model: light, wasm-safe shared types: Level, codes (every
                       diagnostic code), Diagnostic values (Span, Note, RuleRef, DiagEdge),
                       FnTree/TreeNode, change (Op, OpRecord, Entry, Plan, DiffEntry, Task, ...)
crates/core/facts/     brickie-facts: the cache: model (cache.json schema), files (tree walk), hash,
                       lock (`.brickie/cache.lock`, one writer at a time)
crates/core/measure/   brickie-measure, folder bricks:
  workspace/           metadata, rust_analyzer, lock, git, crates
  scip/                SCIP -> rows (normalise, targets, written, modules, layout)
  run/                 mod (run, run_with: progress steps, cache lock), files, edges, items + reach (declared-interface candidates),
                       complexity (brickie-metrics -> FnTree)
crates/core/change/    brickie-change: change sessions (folders: store (base/, ops.json, lock,
                       readers, layout), edit (toml_edit documents, ops, moves, ungroup), session
                       (open, record, review, retract/reapply, replay, editor helpers, links and
                       link check), preview (FileSource over the docs, planned bricks, file
                       sheets for the editor's tables), report
                       (status, diff, todo, added rules: planned edges, no reason), publish (plan,
                       scaffold)); the op/record/report types are plain data in
                       `brickie_model::change` so `crates/web/api` can share them
crates/core/check/     brickie-check, folder bricks:
  diag/                model (Finding -> Diagnostic), summary, text
  config/              schema (one level of the tree), selector, load (tree + include, over a
                       `FileSource`: disk or a change's preview), source, validate,
                       member (member matching, specificity), resolve (module -> leaf path)
  rules/               lift (common ancestor, exports), deps, private, cycles (brickie-graph),
                       complexity, unmapped, stale (internal)
  interface/           declared items (exports chain), uses, `interface/unused`, edge sites
  review/              brickie.review.toml read and edit (internal)
  check/               config + cache + review -> diagnostics (shared by check and serve); the
                       facade for rules and review (dep_subject, complexity_limits, set_review)
crates/web/server/     brickie-web, folder bricks (own brickie.toml): routes (-> render, data, code;
                       also the crate root and tests), render, data, code (the code viewer's
                       provider: views, read, marks, nav); build.rs embeds assets/ui/
crates/web/api/        brickie-web-api, wasm-safe (serde + brickie-model): the server/UI port, see
                       "Web api port" below
crates/web/ui/         brickie-ui, folder bricks (own brickie.toml), compiled to wasm: platform
                       (depends on nothing), state (pure; `state/editor/`: ops, hit tests, typed
                       links, sheets, patching) <- components (also the crate root, wasm entry;
                       `components/editor/`: toolbar, panel tabs, inspector/, canvas layer,
                       connect, aim, drop, place, keys, live)
crates/lib/codeview/       the code viewer, independent of brickie (tag lib; own brickie.toml, exports
                       port and host):
  port/                codeview-port: data types, the Provider trait, endpoints, iframe messages
  app/                 codeview-app: the UI (Sycamore, wasm, client rendered): logic (pure),
                       web/ (explorer, editor, quick open, actions, js), static/ (page, Monaco
                       glue, CSS)
  host/                codeview-host: axum router over a Provider; build.rs embeds assets/codeview/
crates/cli/            brickie: the `brickie` binary (main.rs, cli/: clap args + one file per
                       command) and the integration tests (tests/check, tests/llms, tests/measure)
```

Source files stay small and focused (about 300 lines or fewer, tests aside).

## `brickie llms.txt` (implemented)

Prints `docs/llms.txt` (embedded with `include_str!`; no project or config is read). The guide is
the single place that tells coding agents how to read each diagnostic code and what they must not
do. `crates/core/model/src/codes.rs` is the list of codes; the test `guide_documents_every_diagnostic_code`
fails when a code is missing from the guide, so add a code there and in the guide together.

## Cache layout (implemented, schema_version 6)

`.brickie/cache.json` is written next to the config file. Top level: `schema_version`,
`tool_version`, `rust_analyzer`, `workspace_root`, `git` (`commit`, `dirty`; null outside git),
`crates`, `modules`, `files`. There is no brick data: `check` and `serve` resolve bricks from
the configs.

- Ids: a library crate is its package name, other targets are `pkg/kind/target`; a module is
  `<crate id>::a::b`. A crate records `dir`, its package directory (holding `Cargo.toml`)
  relative to the workspace root, which `path:` members match against.
- `files` is keyed by path: `sha256`, `compiled`, `size`, `lines`, `scope`, `module`, `functions` and `edges`
  (outgoing edges with evidence in that file: line, col, end_col, `symbol`, `written`). There is no aggregated edge list:
  `Cache::edges()` derives module edges by (from, to, kind, scope) at read time, which keeps a
  later per-file incremental refresh possible. Every `.rs` file under a crate root's directory is
  recorded; one the compiler never saw (cfg-gated away, no `mod`) has `compiled: false`, a hash
  and no functions, edges or module, so staleness can tell it from an added file. Complexity (`complexity::analyze`) is tree-sitter only.
- `items` per file (schema 6, `run/items.rs`, `run/reach.rs`): every non-test item with `pub` or
  `pub(crate)` visibility, from SCIP `SymbolInformation` (signature, documentation) and the
  defining occurrence: `symbol` (the same readable form as edge symbols), `module`, `kind` (`fn`,
  `method`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`), `name` (`Type::name`
  for associated items), one-line `signature`, `doc` (first paragraph), `line`, `vis`,
  `visible_in` (the module whose subtree can name the item: its visibility, the visibility of the
  modules around it, widened by `pub use` re-exports) and `exported` (public API of a library
  crate: `pub` through `pub mod`s from the root, or the target of a plain `pub use`, globs
  included). Trait members, trait-impl items, fields, variants and modules are not items.
  Visibility and kind are parsed from rust-analyzer's signature text (`pub fn ..`).
- `implements` edges come from the syntax (`impl Trait for X`), because rust-analyzer emits no
  SCIP relationships. Paths (NPath variant) add sequential decisions; see `crates/lib/algo/metrics/src/paths.rs`.
- `params` does not count `self` (clippy convention).

## Check (implemented)

`brickie check [--config brickie.toml] [--format text|json] [--color auto|always|never]`.
Example config: `docs/examples/brickie.toml`. Code: `config/` (load/merge/validate), `rules/` (deps, cycles,
complexity, unmapped, stale, `codes.rs`), `diag/` (diagnostic value and text printer), `check/`
(orchestration, review file, exit code).

- **Config tree** (`config/load.rs`): each `brickie.toml` with its includes is one level
  (`Config`); a brick with `path = "<dir>"` (relative to the declaring file) is a composite whose
  level is `<dir>/brickie.toml` (missing: `config/missing`); its `members` default to
  `path:<dir>/`. A `path` cycle is an error. A composite file may set `exports` (names or
  `{ brick, scope }`; default none; unknown name: `config/unknown-export`). Names, tags and rule
  selectors are local to their file.
- **Config files**: `include` paths are relative to the including file, recursive, cycle = error; a file
  reached twice is merged once. Lists (`[[brick]]`, `[[deps.rule]]`, `[[complexity]]`,
  `exports`) are concatenated; `deps.default` / `deps.cycles` from the including file override
  includes. Errors (exit 2): duplicate brick, unknown tag/brick in a selector, unknown kind,
  unknown fields (`deny_unknown_fields` everywhere), missing include.
- **Resolution** (`config/resolve.rs`, `config/member.rs`): every module gets a leaf path
  (`["web", "api"]`). At each level the most specific member wins: `path:` < package < module
  prefix < exact `module:<id>`; a longer path or module id beats a shorter one; equal specificity across bricks is
  `config/overlap`. Below a composite the children must partition its modules: a module in no
  child is `config/unassigned`, a child member matching a module outside the composite is
  `config/member-outside-parent`. At the root an unmatched module is only unmapped. All
  `config/*` errors exit 2 with the code at the start of the message.
- **Levels**: an edge between distinct leaves is lifted to their lowest common ancestor `L`
  (`rules/lift.rs`); the siblings `X -> Y` under `L` are checked against `L`'s rules and
  default. Each finding has `brick_level` (`root`, `root/web`), its `bricks` names are children of
  that level, and `rule.loc` names the file that owns the rule (the level's file for
  `deps.default` and `deps.cycles`).
- **Visibility** (`deps/private`, `rules/private.rs`): for each composite on the target's path
  below `L`, the next brick must be in its `exports` for the edge's scope (a plain entry counts
  for all scopes, a scoped entry only for that scope); the first failure is one error per module
  edge, reported at the composite's level. Test edges have no exemption.
- **Deps rules**: a rule's specificity is brick = 3, tag = 2, `*` = 1 on each side; the rule
  with the highest sum wins, on a tie deny wins. `kinds` default all; `scope` default `prod`
  (`prod|test|all`). No matching rule: `deps.default` (`allow` when absent). Edges inside one
  brick and edges touching an unmapped module are ignored. One finding per module edge
  (from, to, kind, scope), evidence = every reference.
- **Cycles** (`cycles = "deny"` in a level's file): per level, SCCs of size > 1 over the lifted
  prod edges among its children, all kinds; one error per SCC with a cycle path note.
- **Complexity**: the limit is inclusive (fails when value > limit). Per function and metric, the
  most specific rule that sets that metric applies (brick > tag > `*`; lower limit on a tie), so a
  brick rule overrides a tag rule only for the metrics it sets. `scope` default `prod`;
  `level` default `error`. One finding per (function, metric). Functions in unmapped modules are
  skipped. Inherited: on each level of a function's leaf path the level's own rules select the
  child on that path; across levels the tightest limit wins, so a nested file can only tighten.
- **Unmapped modules**: one `bricks/unmapped` warning per module not in any brick.
- **Interfaces** (`interface/`, spec `docs/proposals/brick-interface.md`): a leaf declares the
  cached items of its modules that code outside the leaf can name (`exported`, or a
  `visible_in` subtree that reaches a module of another brick); a composite declares the union
  of its exported children's interfaces (recursively). A use of an item is a reference edge (not
  `reexports`) into its module whose symbol is the item or a member of it (fields, methods,
  variants, impl items; `impl` segments and generics are ignored when matching), from outside
  the brick or from test code anywhere; uses are grouped by the using brick's path cut to the
  brick's depth (`<brick> (tests)` for its own tests). `interface/unused` (warn): one finding per
  leaf and declared item with no use; subject `leaf:module:symbol`, span the definition. Edges
  inside one module are not recorded, so a test in the item's own module does not count.
  `edge_uses(from, to)`: every reference site (any kind and scope) from modules under one brick
  path into modules under another, grouped by the most specific covering item.
- **Staleness** (`warn` `stale/file`; `rules/stale.rs`; config edits are never stale): candidates from
  `git diff --name-only --relative <commit>` and `git status --porcelain`, confirmed by sha256;
  without git or with a missing commit all cached files are hashed and crate source dirs walked
  for added files. A cache measured on a dirty tree also hashes all cached files (a file edited,
  measured, then reverted to HEAD is invisible to git). Added files count only under a crate
  root's directory. Uncompiled files are in the cache (`compiled: false`), so they are compared
  like any other and not reported as added.
- **Findings** are `Diagnostic` values (`diag/model.rs`): `id` (first 8 hex of sha256 of
  `code\0subject`), `level`, `code` (`deps/deny`, `deps/default`, `deps/private`, `deps/cycle`,
  `complexity/<metric>`, `stale/file`, `bricks/unmapped`, `interface/unused`), `brick_level`, `message`, `span`
  (primary), `spans` (more locations), `rule` (`name` + `loc` = file:line in the config), `help`,
  `notes` (each with an optional span), `accepted`. A span is `file`, `line` (0 = whole file),
  optional `col`/`len` (1-based UTF-8 bytes) and `label`. Each rule type implements the `Finding`
  trait; the text output is a cargo-style rendering of the value (snippet with carets,
  `= rule/help/note`, three extra locations then "and N more", summary line, pluralised:
  "1 error, 3 warnings"); JSON is its
  serialisation under `{summary, diagnostics}`, which the webview consumes. Columns come from
  SCIP and are assumed to be UTF-8 byte offsets.
- **Help**: `help = "..."` on a `[[deps.rule]]` or `[[complexity]]` overrides the built-in hint.
- **Review**: `brickie.review.toml` next to the config, `[[accept]] id = "..." reason = "..."`.
  Accepted findings are counted, shown in JSON (`accepted: true`), hidden in text, and do not
  fail the run.
- **Exit code**: 1 when an unaccepted `error` finding exists; 2 on a config, review or cache
  error (including a missing cache or a schema mismatch); else 0.
- Tests: `crates/cli/tests/check/` builds caches in code (no rust-analyzer) and includes a text snapshot;
  `crates/cli/tests/check/nested.rs` covers a two-level tree (a folder brick of three crates with its own
  `brickie.toml`).

## Serve (implemented)

Stack: axum 0.8 + sycamore 0.9, no htmx and no JS framework. Code layout (small focused files,
folders by concern):

- `crates/web/server/src/`: `lib.rs` (options, state incl. the bound address and the measure job, `run`),
  `measure/` (the measure job: single flight, runner), `routes/`
  (`index`, `api`, `interface`, `review`, `editor` (change endpoints), `measure`, `guard` (write guard),
  `view` (page data of a request: current design or change preview), `assets`), `render/`
  (page HTML shell), `data/` (page data from the cache: `diagnostics`, `graph/` (per-level
  aggregation: `edges`, `level`, `heat`), `functions`, `interface`, `editor` (change view,
  planned/proposed marks), `time`), `tests/`.
- `crates/web/api/` (`brickie-web-api`): `endpoint/` (`mod`: the `Endpoint` trait and markers;
  `editor`: the change endpoints; `measure`: the measure job), `model/` (page data and pure
  helpers: `params`, `diag`, `measure` (job state, `age`),
  `graph`, `levels`, `function`, `score`, `boundary`, `interface`, `editor` (change view and
  requests; re-exports `brickie_model::change` as `change`)).
- `crates/web/ui/` (`brickie-ui`): `state/` (`layers`, `selection`, `focus`, `eval`, `shared`,
  `ctx`, `live`, `history`, `events`, `editor/` (`ops`, `hit`, `dnd`, `undo`, `link`, `sheet`,
  `patch`, `words`)), `components/`
  (`app`, `keys`, `call`, `canvas/` incl. `frame`, `panels/` incl. `node`, `controls/` incl.
  `crumbs`, `editor/` (`bar`, `dialogs`, `toolbar`, `panel`, `inspector/`, `rules`,
  `complexity`, `level_tab`, `files`, `log`, `layer`, `connect`, `aim`, `drop`, `place`,
  `overlay`, `keys`, `live`, `widgets`), `popup`), `platform.rs`, `wasm.rs`, `styles/*.css`.

- **Web api port.** `crates/web/api` holds the `Endpoint` trait (`PATH`, `Req`, `Res`) with one marker per
  JSON endpoint (`Review` POST `/review`; `Graph`, `Diagnostics`, `Functions` GET `/api/*`, where `Req` is the
  query `Params` (with `change=1`: the open change's preview); `Interface` POST `/api/interface` `{brick}` and `EdgeUses` POST `/api/edge-uses`
  `{from, to}` with brick paths, 400 for an unknown path), the path consts of the HTML routes, and every type crossing the port (page data,
  params, review request/reply). The server mounts routes at `E::PATH` with handlers typed
  `ReqOf<E>` / `ResOf<E>`; the UI calls `components::call::<E>`, which only moves JSON through
  `platform::post_json`. A contract change fails to compile on both sides. The server still depends on `ui`
  only to render (`render_page`, `CSS`, `APP_ID`, `DATA_ID`). `state` is pure: `Cx::new` takes
  the stage size, and `components::app` mirrors params into the URL (push only when the URL differs from the
  last seen one; popstate records its URL first, so back/forward adds no history entries).

- **One component tree, two runtimes.** The server renders `brickie_ui::app` with
  `sycamore::render_to_string` (output carries `data-hk` hydration keys) into `<div id="app">`
  and embeds the page data as JSON in `<script id="brickie-data">`. The wasm bundle reads that
  JSON, rebuilds the same tree and hydrates `#app`. The page JSON holds all diagnostics, every level of the
  brick tree (bricks, tag rows, aggregated edges per level, see "Levels") and the functions;
  nothing is filtered on the server.
- **Layout runs on the client, once per level.** `crates/lib/algo/layout/src/` is a pure deterministic
  function `layout(Input) -> Layout` (positions of boxes, ports, routed cubic paths); the server
  calls the same function for the SSR page so the no-JS page also shows the graph. It runs once
  from the
  full edge set (prod edges, test-scope edges and violating edges); presets, layers and selection
  only change classes and styles of existing SVG elements (no relayout, no flicker). Dragging a
  brick re-routes with `Layout::path(edge, positions)` without moving anything else.
- **State in the URL.** `Params` (defaults omitted): `p` preset, `lay` layer differences
  (`-cx,+tests`), `vs` violations scope, `brick` (comma list), `group` (tag rows), `edge`
  (`from>to>kind`), `item` (selected finding), `level`, `status`, `code`, `grp`, `fn_sort`,
  `fn_dir`, `fn_scope`, `fn_limit`, `lv` (current level of the tree, `root` by default), `below`
  (`1`: findings of this level and below only), `bnd` (`frame`: outside edges end at the
  frame, see "Levels"). Every link has the real `href`; hydrated, a plain click
  updates the signal and `history.pushState`s, `popstate` re-reads the URL. A full load with
  params renders the same view.
- **Without wasm** (bundle not built) the SSR page is complete but read-only: the canvas, filters
  and links are server rendered; pan/zoom/drag, dropdowns and accept/unaccept need the bundle.
- **Endpoints**: `GET /` (page), `GET /api/diagnostics`,
  `/api/functions` (`brick=` takes tree paths such as `root/web`, matching the subtree),
  `/api/graph` (the levels, JSON, same params), `POST /review` (JSON
  `{id, reason}`; `reason: null` unaccepts), `GET`/`POST /api/measure` (see "Measure job"),
  `/static/ui/brickie_ui{.js,_bg.wasm}`, the code
  viewer under `/code/`. Cache and
  config are read per request (re-run `measure`, refresh).
- **Measure job.** `POST /api/measure` (behind the write guard) starts `brickie-measure` in
  the server process (`run_with` on tokio's blocking pool, reporting each step), or returns the
  job already running: one job at a time (single flight). `GET /api/measure` answers
  `{state: idle}`, `{state: running, step, started}`, `{state: done, at}` or
  `{state: failed, error, log}` (times in Unix seconds; `log` is the tail of the steps and the
  error chain, at most 20 lines). The header shows the cache age and the stale-file count
  (`CacheInfo.age_secs`, `stale_files`: "measured 4m ago", "stale: N files") and a Measure
  button; the job's progress arrives as `measure` events (below; without `EventSource` the
  button polls `GET` once a second) and shows the step; a finished measure raises the "Code
  re-measured · Refresh" pill instead of reloading. After a successful publish the editor
  offers "Measure now".
- **Events (SSE).** `GET /api/events` (`text/event-stream`, read-only so no write guard, a
  keep-alive comment every 15 s) streams typed `ServerEvent`s (`crates/web/api/src/model/
  events.rs`; `event:` = kind, `data:` = the JSON): `change {open, head}` when the change log
  (`.brickie/change/ops.json`) moves or a change is opened / published / discarded, from any
  writer (page, CLI, agent); `cache {measured_at, stamp}` when `cache.json` is replaced (CLI
  `brickie measure` included); `measure {job}` on every job state change; `config {stamp}` when
  a `brickie*.toml` of the tree changes. One watcher per server (`crates/web/server/src/
  events/`) fans out over a `tokio::sync::broadcast` channel. Decision: a cheap mtime watcher
  (a few stats every 200 ms, the toml tree walked every 2 s; hidden folders and `target`
  skipped) instead of the `notify` crate: no new dependency, no inotify limits, the same on
  every platform and filesystem, and the atomic renames that write `ops.json` / `cache.json`
  show as a new mtime. Reconnect: every connection first gets the current state (one event of
  each kind), so the browser's automatic `EventSource` reconnect resyncs without
  `Last-Event-ID` replay. The page (`components/editor/live.rs`, pure decisions in
  `state/events.rs`) keeps one `EventSource`: `change` with a new head fetches `/api/change` and
  the levels and patches in place (toast "Agent proposed N ops"; retried every 400 ms while a
  gesture or a typed name is in progress); a change opened or closed elsewhere, a new `cache`
  stamp and (with no change open) a new `config` stamp raise a pill with Refresh (the first
  stamp of a connection is the baseline). Without `EventSource` it polls `/api/change` every
  3 s. Measured: an agent's `brickie change propose` shows on the page about 0.8 s later (release
  build; the event itself arrives in ~50 ms, the rest is the two fetches).
- **Cache lock.** Every measure (CLI or server job) holds `.brickie/cache.lock`
  (`brickie_facts::CacheLock`, the holder's PID inside) for its whole run, and writes
  `cache.json` through a temp file and a rename. A second measure fails at once with "another
  measure (process N) is writing the cache; wait for it to finish and retry" rather than
  waiting: a measure takes minutes, and a waiting CLI looks hung. A lock whose process is gone
  (crash) is taken over.
- **Diagnostics** are `check`'s `Diagnostic` values field for field (`DiagView`), plus a snippet
  with the span marked and `bricks`. A selected finding (`item=`) shows the same card as before
  (code, message, spans as source links, rule, help, notes, Accept/Unaccept).
- **Code viewer** (`/code/`, [proposals/code-viewer.md](proposals/code-viewer.md),
  implemented): the generic `codeview` component mounted with
  `nest_service("/code", codeview_host::router(CodeProvider))`. The source popup opens it in its
  iframe (`/code/?path=&line=&col=`, built by `code_href`; every source link of the UI, function
  links included, is such a URL); on the viewer's `Ready` message the brickie UI sends `Filter`
  (the explorer scoped to the single selected brick); a mark's
  "finding" link posts `Link` back, which selects the finding (`?item=&lv=`) and closes the
  popup. Every brick's details panel has an "Open code" link
  (`/code/?filter=<level path>&view=Bricks`; the viewer then opens the brick's entry file).
  Endpoints (`POST /code/api/{views,file,marks,bands,definition,references}`, JSON) are
  answered by `crates/web/server/src/code/` from the config, cache and check result per request:
  views "Bricks" (brick tree, leaf -> crate -> folders -> files; nodes carry a `NodeKind` and the
  brick's tags, rendered as marker, colour and chips) and "Folders", badges = open errors and
  warnings by primary-span file summed up; files only when listed in the cache and confined to
  the workspace, `stale` when the hash differs; marks = every span of a finding in the file
  (accepted ones as info); bands = for every measured function of the file one band over the
  function (depth 0, heat = its worst metric / limit, capped at 1; detail lists every limited
  metric) and one per node of its decision tree (`brickie_measure::fn_tree`, tree-sitter,
  computed on demand per request, nothing cached; depth = nesting, heat = depth / max(nesting
  limit, deepest)); definition = the recorded item of the reference edge (or item
  declaration) under the cursor, else its module file; references = the edges to the same
  symbol, labelled with the using brick, other bricks first. The viewer: explorer with view
  switcher and "show all", tabs, Monaco read-only (theme follows `prefers-color-scheme` or
  `Theme`), marks as Monaco markers plus a list under the editor, bands as nested bars in
  Monaco's line-decorations gutter (one 6 px column per depth, colour cold to hot by heat, the
  gutter widens to the deepest band; hover shows label, lines and detail; click highlights the
  band's lines and reveals them; a "bands" toggle in the tab bar hides them), F12 / Shift+F12 (one
  definition opens directly, otherwise a results list; cross-file opens a tab), Ctrl+P quick
  open. Bundle: `scripts/build-codeview.sh` (called by `build-ui.sh`) fetches monaco-editor
  0.52.2 from the npm registry (sha512 checked) and keeps only `loader.js`, `editor.main.{js,css}`,
  the worker bootstrap, the Rust grammar and the codicon font (4.4 MB), builds the app (about
  0.5 MB wasm) and copies the page into `assets/codeview/` (git-ignored, 4.8 MB in all);
  `codeview-host/build.rs` embeds every file there, and without them `/code/` is a page saying
  how to build it. No CDN at runtime. Deviations from the proposal: no hover yet, no complexity
  info marks (complexity is shown as bands instead), endpoints are all `POST` under `api/`, `Node` has an `id` (the `Filter` target)
  and `ToHost` a `Ready`.
- **No source page**: the server-rendered `/source` page (and its complexity gutter) was
  removed; its tree now lives in the code viewer as bands. Source links are plain
  `/code/?path=&line=` hrefs, so without JavaScript they open the code viewer in a tab, which
  itself needs JavaScript (accepted).
- **State styling.** Colours are CSS variables in `crates/web/ui/styles/theme.css` (light and dark), rules in
  `crates/web/ui/styles/state.css`; classes only change styles, never geometry, so presets do not relayout.

  | state | edge | brick |
  | --- | --- | --- |
  | allowed | light grey, thin | grey border, plain fill |
  | violating, error | dark red, thick, soft glow | thick red border, light red fill |
  | violating, warn | orange, medium | orange border and fill |
  | accepted | red, dashed, faded | dashed border |

  Edge-kind colours show only in the Architecture preset (class `kc`); in All and Violations
  allowed edges (and their arrowheads) are neutral grey, and the kind stays readable by arrow and
  dash shape. Edges that can violate (they carry `deps` findings or lie in a cycle finding) are
  rendered in a second SVG group above the others; the split is static, never changes with state.
- **Brick score** (`crates/web/api/src/model/score.rs`, the one place scores are computed): `score = open
  errors + 0.25 x open warnings + sum over functions of max(0, worst ratio - 1)`. A finding is
  open when its level is `error`/`warn` and it is not accepted, whatever layers are shown. A
  function's ratio is its worst `value / limit` over the metrics that have a limit under the rules
  that apply to its brick and scope (`check::complexity_limits`, the same lookup as `check`).
  A brick's `desc` (config, one line) travels on `BrickView.desc`: the brick tooltip shows it under
  the name and the details panel at the top (leaf, composite, faded neighbour).
  **Interface** (`panels/details/iface.rs`): the brick details (leaf, composite or neighbour)
  end with the brick's declared interface, fetched on selection (`Interface`): `path · N items ·
  M unused`, then one collapsible row per item (kind, name, use count or a `⚠ unused` pill, first
  doc line; most used first, unused last). Open, it shows the signature (a link to the
  definition) and the uses grouped by using brick, each site a source-popup link with the
  written text and a `test` / kind tag. A selected edge (allowed or violating, also a frame or
  neighbour edge: the frame end is the current level's composite, so the sites are the union of
  the underlying module edges) shows **Code behind the edge** (`EdgeUses`): `N items · M sites`,
  one open group per used item of the target (signature link, or `in <module>` for an item that
  is not part of any interface) with every reference site. `interface/unused` findings form the
  `interface` group in the finding layers (off in the Architecture preset).
  The server embeds the static part in each `BrickView` (`worst_ratio`, `hot`: functions over a
  limit, worst first); the page adds the open counts (`brick_scores`), so scores follow
  accept/unaccept. The score appears as a corner badge (`3E 1W`, or a green check when clean), a
  heat bar under the brick name (length = ratio / 2, full at 2x the limit; colour by level: below
  0.8, 0.8-1, 1-1.5, over 1.5), the brick tooltip, the "bricks by tag" list (worst first inside
  each tag, ordered by the score at load) and the scorecard in the details panel (open findings
  per deps / cycle / complexity / stale / interface, the worst 3 functions with metric, value/limit and ratio
  as source links, in/out edge counts with violating ones). Bricks are 36 px wider to fit the
  badge. Tests: `model::score::tests`, `web::data::graph::tests`.
- **Cycle edges.** `Diagnostic.edges` (`DiagEdge {from, to, span}`) lists the brick edges a
  finding is about; `deps/cycle` carries every prod brick edge inside the strongly connected
  component (each lies on some cycle; the `cycle: a -> b -> a` note is one example path), with the
  first reference of each pair as evidence. `check` prints one `is on a cycle, used at` note per
  edge; the primary span is the first edge. `web::data::graph` adds the cycle's id to the `diags`
  of the matching prod `EdgeView`s (all kinds of that brick pair), so cycle edges follow the same
  path as deny edges: drawn `err` (red, thick, top group), counted as violating in the scorecard,
  highlighted when the finding is selected. Tests: `crates/cli/tests/check/cycles_complexity.rs`,
  `web::data::graph::tests`, `state::eval::tests`.
- **Counts and explanations.** The header counts findings of the layers that are on and says how
  many are hidden by layers; the findings list reads `Findings (n of total)` (layers, scope, chips
  and text filter narrow it; tooltip). Brick badges, scores and the scorecard count every open
  finding whatever layers are on (tooltips say so; a cycle counts once per brick in it). The
  banner shows the last three path components, the full workspace path is its tooltip. Brick names
  longer than the box are cut with an ellipsis (tooltip has the full name); tag row captions stay
  about 12 px on screen whatever the zoom. `/static/ui/*` is served with `Cache-Control:
  no-cache` so a rebuilt bundle is never stale.
- **Review**: `POST /review` edits `brickie.review.toml` next to the config with `toml_edit`
  (comments and other entries kept; a reason is required; the id must be a current finding to
  accept); `check` reads the same file.
- **Building the bundle**: `scripts/build-ui.sh` (needs `rustup target add
  wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.123 --locked`,
  matching `crates/web/ui/Cargo.toml`) builds `brickie-ui` with `--features hydrate` for
  `wasm32-unknown-unknown` and runs `wasm-bindgen --target web` into `assets/ui/`
  (git-ignored). `crates/web/server/build.rs` sets `cfg(ui_wasm)` when both files exist, and the server then
  embeds them; otherwise it builds anyway (with a cargo warning) and serves the read-only SSR
  page. Release/CI order: `scripts/build-ui.sh`, then `cargo build --release`.
- **sycamore 0.9 notes**: `render_to_string` always emits `data-hk` (needed here); element
  event handlers need `web_sys` event types even on the server target; `view!` treats any
  expression other than a plain identifier as dynamic (a closure that must be `'static`), so
  values are bound to locals first; svg attributes are set with quoted names (`"viewBox"=..`);
  the wasm is about 1.2 MB without wasm-opt.
- **Known limits**: measured time is the cache file's mtime; accept needs the wasm bundle; source
  links need JavaScript (the code viewer); the layout is not cached (about 25 ms natively for 15 bricks
  and 33 edges in release; the SSR page recomputes it per request).
- Tests: `crates/web/server/src/tests/` (tower `oneshot`: index, layers and presets as params, JSON
  endpoints, code viewer adapter (trees, files, marks, bands, navigation), review round trip,
  missing cache); `crates/lib/algo/layout/src/tests.rs` (no edge crosses a box, boxes do not overlap, determinism,
  crossings at most the config order, rows follow tags, empty graphs, re-routing after a drag);
  `crates/web/ui/src/state/` (layers round trip through the URL, presets, selection,
  scoping); `crates/lib/algo/metrics/src/tests.rs`; `codeview-host` tests (endpoints, bands, path
  confinement).

### Design editor (implemented)

Edit mode of the viewer over a change session (UX spec: `docs/proposals/editor-ux.md`; change
model and endpoints: `docs/proposals/editor.md`). `change=1` shows the open change's preview
(the same level aggregation over `Preview::check`; `BrickView.planned` / `proposed` mark bricks
with no code yet and agent proposals). The header button **Design** (`E`) opens a change (intent
dialog) or toggles edit mode; the change bar replaces the banner (chips that act: proposed /
conflicts / without reason open the Ops or Rules tab, planned selects the planned bricks;
Preview | Current | Diff with `P`/`C`/`D`; the measure age; Publish shows blockers first, then
the files and the warnings).

- **Layout**: the left column stays the findings panel; the right column is the design panel
  (`components/editor/panel.rs`) with tabs Inspector, Rules (`[deps]` default / cycles, the rule
  table of the level's file and its includes, each row with its physical file, tinted when
  included; an add row with the level's selectors; the complexity table, one auto-applying cell
  per limit), Level (exports with scope pills), Files (includes, "New preset" = `create_file` +
  `add_include`, shared-file warnings) and Ops (grouped by element, plain words, author / state
  chips, filters agent / human / proposed / conflict, "Accept all from agent"). A floating
  toolbar sits at the top of the canvas (`editor/toolbar.rs`, collapsible, outside the pan/zoom
  transform): Select / Hand (`V`/`H`), the brick tiles Crate / Module / Group (drag with a ghost
  or click then click the canvas; `B`, `Tab` cycles), the link types uses / implements /
  reexports / deny (`L`: ports on every brick), the scope of new rules prod / test / all
  (`1`/`2`/`3` in edit mode) and delete.
- **Canvas** (`editor/layer.rs`, inside the pan/zoom group): typed ports (one per link type,
  own colour and glyph) on the hovered or selected brick; dragging one draws a rubber band,
  outlines the siblings and checks the target under the pointer with `/api/change/link-check`
  (refused targets read "already allowed"); the drop records `add_rule` at once, selects the
  rule and focuses its reason (`editor/connect.rs`). A tag lane (row) is a `tag:<t>` target.
  Planned edges (`LevelView.planned_edges`) are dashed in their type's colour, deny red, agent
  proposals violet with an `A`, a "?" without a reason; a click selects the rule. Bricks touched
  by pending agent ops carry an `A` badge, conflicting ones `!`. A new brick is named inline on
  its box (`editor/overlay.rs`; module members prefilled `crate::`), `F2` renames inline, `Del`
  opens a confirmation listing the rules and exports that name the brick.
- **Drag and drop: folders and lanes** (`state/editor/dnd.rs` pure, `editor/aim.rs`,
  `editor/drop.rs`, `editor/place.rs`; interaction tables in `proposals/editor-ux.md`). What a
  drag would do is decided under the pointer with one precedence: brick body > "+ new tag lane"
  zone > frame edge / breadcrumb (existing bricks only) > tag lane > empty canvas. A brick body
  is a folder: highlighted at once (no hold), with a lid tab and "into web (3 inside)" or
  "(becomes a group)"; release moves the dragged brick into it (`move_brick`, which makes a
  leaf composite) or, for a new tile, makes it composite if needed (`make_composite`, path =
  its name) and adds the brick in its level file. A tag lane (rows are lanes; between bands the
  nearest row counts) retags a dragged brick (`set_brick {tags}`: its first tag replaced by the
  lane's, the others kept; the untagged lane drops the first tag) or tags a new tile; a lane
  drop lets the layout place the brick (its stored position is cleared). Its own lane only
  stores the position. The new-lane zone asks a tag inline. Faded neighbours and the brick
  itself are refused (red, not-allowed cursor, reason). The ghost / dragged brick shows the
  result ("tag: lib", "into web", "up to root", "move here"); `Esc` cancels a drag and puts
  the brick back. The inspector has Make group / Ungroup.
- **Undo / redo** (`state/editor/undo.rs`): `Ctrl+Z` / the change bar's Undo retracts the
  latest human op (`/api/change/retract`); `Ctrl+Shift+Z` / `Ctrl+Y` / Redo restores the latest
  retracted one (`/api/change/redo`, a `restore` review; refused with 409 when it no longer
  applies or would put a later op in conflict). The redo stack comes from the log
  (`ChangeView.redo`, `brickie_change::redo_stack`: retracted human ops since the last new human
  op). A gesture that recorded several ops (tile into a leaf: `make_composite` + `add_brick`)
  is kept as a `(first, last)` seq group in the page and is undone and redone whole. The
  buttons' tooltips name the op ("Redo: Made cli a group (2 ops)").
- **Back / Forward**: `←` `→` next to the breadcrumb, `Alt+Left` / `Alt+Right` (not while
  typing) and mouse buttons 4 / 5 call `history.back()` / `forward()`, so the page and the
  browser history stay in step (`state/history.rs` tracks the entries the page pushed to know
  what each button would show: "Back to root/core"; disabled when there is none).
- **Inspector**: brick (review bar for its pending agent ops; name, description, tag pills,
  member chips, group path; each applies on blur / Enter as one op, no Save), rule (kinds,
  scope, action, reason, help, file and shared-by count, Delete), measured edge (the deciding
  rules of the level, linked) and, with nothing selected, the level (file, deps, tags).
- **Review**: the review bar has Accept / Reject (a note is required to reject) and, for a
  conflict, "Re-apply as mine" (`/api/change/reapply`); `A`/`R`/`N` accept, focus the reject
  note, jump to the next proposal (its level, selected and centred). Ops written elsewhere
  (the agent's `brickie change propose`) arrive through `/api/events` (see Serve, "Events"); a
  toast tells how many proposals came.
- **In-place patching**: every write answers `ChangeReply {done, change, levels}`; the editor
  replaces the touched levels (`state::patch_levels`, levels missing from `change.files`
  dropped), swaps the change and re-renders the level on screen without a reload
  (`state::Live`: levels, change and a revision signal), keeping pan/zoom and the boxes of
  bricks that were on screen (`state::Keep`); only new bricks get their drop point or an
  automatic place. Selection lives in the URL params and the editor's rule selection.
- **Server addition**: `ChangeView.sheets` (`FileSheet` per config file of the preview: its own
  deps settings, rules, complexity rules, exports and includes, in file order, so a row maps to
  an op's `file` and `index`; `Preview::sheets` in `brickie-change`).
- **Spring-loaded zoom** (`editor/spring.rs`): a drag (brick, tile or lifted brick) resting
  on a group brick for 700 ms opens its level (the folder fill charges meanwhile, "· hold to
  open"; none with `prefers-reduced-motion`). A brick dragged on its own level is lifted off it
  first (`Carry::Brick {level, name}`): it goes back to its place there and its ghost keeps
  following the pointer on the new level, where it can go into a brick (or rest to go deeper),
  up (frame edge, breadcrumb) or onto the level (`move_brick`). Back on its own level it acts
  as a brick drag again.
- **Cut / paste** (`editor/clip.rs`): `Ctrl+X` marks the selected brick (hint "Cut X ...");
  `Ctrl+V` on another level moves it there, or into the selected brick (`move_brick`); same
  level without a selection, or into itself, is refused with the reason; `Esc` forgets it.
- **Retarget** (`editor/retarget.rs`): the selected planned rule shows a handle on each
  arrowhead (drawn above the port strips); dragging it draws the rubber band from the rule's
  source and the drop records `set_rule` with only `to` changed (the rule stays selected).
- **Writes that break the design**: a write whose preview loads but fails its checks (e.g.
  `config/member-outside-parent` after a move) is recorded; its reply carries no levels and
  `change.error` says why (paths relative to the root config's directory), shown at the
  gesture ("Recorded, but the design no longer checks ... · Ctrl+Z undoes").
- **Discoverability**: `?` (or the toolbar's `?`) opens a sheet of every key and gesture; the
  Open-change dialog focuses the intent field (Enter opens, Esc cancels); the "already
  allowed" refusal names the link and the deciding rule by its relative location
  ("No rule added for web → lib: already allowed by brickie.toml:62").
- **Deviations from the spec**: the toolbar is collapsible but not draggable; a tile click then
  a click on the canvas places a brick (no drag-a-box sizing); double-click on a leaf does not
  rename (use `F2` or the inspector); a lifted brick dropped on another level lands untagged
  wherever the layout puts it (no lane retag in the same gesture); the exports strip on the
  frame and drag-a-tag-swatch are not implemented (exports and tags are edited in the Level
  tab and the inspector); members have no autocomplete; Diff lists the semantic diff (before /
  after per element) over the preview rather than overlaying both designs; the measure prompt
  is the header's measure control and the change bar's "code measured N min ago".

All writing endpoints, `/review` and `POST /api/measure` included, pass one write guard (`routes/guard.rs`): loopback
bind, loopback `Host` with the server's port, same `Origin`. Canvas positions are layout hints
in the change, never ops.

### Levels (implemented)

Map-style zoom over the brick tree (spec: `docs/proposals/nested-bricks.md`, "View").

- **Server, per level** (`crates/web/server/src/data/graph/`): one `LevelView {path, file, exports, graph}`
  per level (root first, then depth first). Its `graph` is a `GraphView` of the level's
  children (rows by first tag, config order) plus the faded outside neighbours. Every module
  edge with at least one end inside the level is lifted: an end inside maps to the child that
  contains it, an end outside to its path cut one below the common ancestor with the level
  (`End::Outside`, the sibling of the nearest enclosing level), as in the prototype. Edges are
  aggregated by (from, to, kind) with `count` (evidence sites), `diags` (the `deps/*` finding ids
  of the module edges plus the cycle findings of this level), `worst` (`error`/`warn`) and
  `wall` (a `deps/private` finding reported at this level on an edge coming from outside).
  Bricks carry `path` (`root/web/api`), `children`, `exports`, `file`, `exported` (by this
  level) and `ghost`; heat (`worst_ratio`, `hot`) covers the whole subtree. Functions carry their
  leaf path. Tests: `web::tests::levels`, `model::levels::tests`.
- **UI**: `Params.lv` picks the level; the app keeps the shared state (`state::Shared`: params,
  review state, popup, menus) and rebuilds the page from a fresh `Cx` (layout, pan/zoom, drag)
  when the level changes. Window handlers (keys, pointer, resize) are registered once and act on
  the current `Cx` (`state::Cur`). Findings land on the brick of the level on screen that
  contains them (`model::diag_bricks`), so a composite's badge and score count its subtree.
- **Drawing**: a frame around the children labelled `name · exports [..] · file`; composites
  have a double border and their exported children as ports on the bottom border, a child the
  level exports a port on its top border; neighbours are faded and dashed. Edge width grows
  with `count` (CSS `--k`), edges with findings are red as before; a `wall` edge is cut where it
  meets the frame and ends with a cross. Hover (SVG title) peeks into a composite: its children,
  exported ones marked.
- **Outside edges** (`bnd`, toggle "Outside → frame | children" next to the breadcrumb, hidden at
  the root; `components/controls/boundary.rs`): `children` (default) draws each boundary edge to
  the child it reaches; `frame` draws one edge per (neighbour, direction, kind) to the nearest
  frame wall. The frame edges are aggregated on the client (`api` `model/boundary.rs`, `frame_edges`:
  counts summed, findings united, worst level, `test` only when all are, `wall` when any is)
  and appended after the level's edges in `Cx` (flagged by `EdgeView.under`, the boundary edges
  they stand for; not serialised); the layout is computed without them, and the evaluation
  shows one of the two sets, so switching changes only params (no rebuild, pan/zoom and drag
  kept). A frame edge's route is one cubic from the neighbour to the frame wall
  (`canvas/boundary.rs`), spread per neighbour, and a `wall` one is cut at the wall as usual. Its
  inside end is named `[frame]` in the `edge=` key; selecting it scopes the lists to the
  neighbour and the children it stands for, and its panel lists all underlying findings. Brick
  scores and brick edge lists ignore frame edges (they repeat boundary edges). Test:
  `model::boundary::tests`.
- **Composites** have a tinted fill (`--comp-fill`; error and warning fills still win), so
  they read as "has a level inside" before the zoom icon is noticed.
- **Free layout**: dragged bricks stay where they are dropped until the level changes; the
  header button **Auto layout** (disabled while nothing is dragged) puts them back and refits.
  The page shell is rebuilt only when the level changes (`create_selector` on the level
  index), so selections and filters keep pan/zoom and drag state.
- **Navigation**: double-click on a composite zooms in, as does a click on its zoom icon (a
  small magnifier with a plus at the start of every composite's tag line, `canvas/zoom_icon.rs`; no key or
  panel link). Double-click is detected from two clicks on the same brick within 450 ms, since
  the first click re-renders the brick and the browser never fires `dblclick`. On a neighbour it opens its enclosing level; Backspace zooms out, Esc closes the popup, then the finding, then the
  selection, then zooms out; the breadcrumb (`root › web`) and the browser back button work
  through the URL. The breadcrumb is an overlay at the top-left corner of the canvas (not in
  the header): outside the pan/zoom transform, on a panel background, its clicks do not pan
  or clear the selection. The canvas plays a short scale transition when a level mounts (`zin`/`zout`),
  none under `prefers-reduced-motion`.
- **Findings**: each row shows `@ level`; clicking one jumps to its `brick_level` and selects
  it (its edge is highlighted there); the `levels` chip "this level and below" filters the list
  (findings without a level count as root).
- **Deviations from the spec**: the level lives in the query (`lv=`) like every other view
  state, not in the hash, so the server renders the same level; outside neighbours are drawn in
  an extra first row (`inbound`: sources) and last row (`outbound`: targets) instead of left/right columns, because the
  layout engine is row based; edges stay aggregated per (from, to, kind) so kind colours keep
  working; clicking an edge lists its findings (with their evidence spans) but not every module
  edge, which the page data does not carry; the peek is the SVG tooltip.

## Port of the prototype viewer

Reference: the prototype's `index.html` (JS, layout and UI), not published.
The port keeps the prototype's CSS variables, colours, edge styles, legend, header controls,
three-pane shell and interaction behaviour; only data that no longer exists was remapped.

### Layout: original JS function -> Rust (`crates/lib/algo/layout/src/`)

| JS function / constant | Rust | Deviation |
| --- | --- | --- |
| `LAY`, `BZ` | consts in `layout/mod.rs` (`H`, `GAPY`, `GAP`, `DUM`, `PADL`, `PADR`, `ITER`, `RESTARTS`, `REFK`, `LANE`, `PORT`, `PAD`) | `GG`, `FAMW` dropped (families) |
| `computeLayout` | `compute::compute_layout` (+ `rank::build`) | no areas/side columns/`zch`; rows are tags; initial y 40 instead of 72 (no zone panel labels); `fam` ordering is brick (config) order |
| `layerCross` | `order::layer_cross` | no side-column pseudo endpoints |
| `totalCross` | `order::total_cross` | no family penalty |
| `pc`, `pairCost` | `order::pc`, `order::pair_cost` | computed without the `_nb` cache |
| `transposeAll` | `order::transpose_all` | no family term |
| `siftRow`, `siftAll` | `order::sift_row`, `order::sift_all` | no family term |
| `annealOrder` | `anneal::anneal_order` | exact-integer LCG (JS multiplied in doubles); no family term |
| `sweep` | `order::sweep` | no side-column endpoints |
| restart loop, `keepBest`, final sift/anneal in `computeLayout` | `order::minimise` | `best` is cloned (JS aliased arrays) |
| `pava` | `place::pava` | none |
| `Wd`, `Wp` | `rank::wd`, port-count widening in `rank::build` | none |
| `sep`, `place` | `place::sep`, `place::place`, `place::place_x` | `GG` branch dropped |
| arcs (`lvl`, `maxL`) | inside `compute::compute_layout` | none |
| initial ports (`pl`, `addp`, `po`) | `ports::initial_ports` | no side-column ports |
| `bzPort`, `bzCtl`, `bzSample`, `polyLen` | `geometry::bz_port`, `bz_ctl`, `bz_sample`, `poly_len` | none |
| `polyCross`, `polyNear` | `geometry::poly_cross`, `poly_near` | no spatial grid (`segGrid`, `GC`) |
| `smartRoute` | `smart::smart_route` | `getTotalLength` sampling replaced by per-segment sampling; the `prim` priority is uniform (all edges are primary); gutter-lane port ordering dropped |
| `routePath` | `path::route_path` / `route_with` | no side-edge (`roundedPoly`) or hidden-edge fallback; returns cubic segments |
| `mkItem`, `measureItems` | `measure::mk_item`, `measure_items` | used by tests and `score_layout` |
| `scoreLayout` | `measure::score_layout` | takes edge indices; no scratch SVG |
| `layoutShown`, `layoutFor` | `layout()` | one candidate (no base-edge variant, no `free_use` bricks) |
| `fitView`, `applyView`, wheel/pan handlers | `ctx::fit_view`, `components::canvas` | none |
| `renderGraph` (edges, nodes, row captions) | `canvas::edge::edge_view`, `canvas::brick::node_view`, `canvas::rows::row_labels` | static elements with reactive class/style instead of re-rendering |
| `defsHTML` | `canvas::defs::defs` | `composes` markers used for `reexports`; no `extends` |
| `edgeOn`, `violIds`, `fOn`, `vOn`, `focusSpec`, `fSpec` | `state::evaluate`, `state::finding_focus`, `state::f_on`, `v_on` | see data remap below |
| `LDEF`, `PRESETS`, `SUBS`, `applyLay`, `layStr`, `setPreset` | `state::layers` (`LDEF`, `preset_over`, `SUBS`, `Lay::from_params`, `Lay::diff`, `with_preset`) | layers remapped |
| `selSet`, `scopeSet`, `scopeName`, `onBrick`, `addBrick`, `selectGroup`, `applySel` | `state::selection::Sel` methods | group = tag row |
| `listFindings`, `renderLeft`, `fRow` | `state::list_findings`, `panels::diagnostics::left`, `finding_row` | no sort/rank selects, no text-free `sub` select (code chips) |
| `renderDetail`, `summaryHTML`, `brickHTML`, `groupHTML`, `edgeHTML`, `findingHTML` | `panels::details::detail` and siblings | content remapped |
| `renderLegend` | `panels::legend::legend` | kinds and states of the current data |
| `vertHTML`, `iceCol`, `heat` | `web/server/src/code/bands.rs` + code viewer bands (`glue.js`) | every function of the file as bands in Monaco's gutter; no line tint, no horizontal "ice" tab |
| `fn_tree_source` (`cc.py`) | `complexity::fn_tree` | flat pre-order nodes |

### Data remap (Brickwork concept -> current data)

- Layers (rows) -> tags in config order (first tag of a brick); zones, areas, families, side
  columns, sockets, `free_use` bricks and capabilities do not exist. A tag row caption selects
  the row (the prototype's layer/family/zone group selection).
- Edge kinds `uses`, `implements`, `extends`, `composes` -> `uses`, `implements`, `reexports`
  (`reexports` takes `composes`' dotted line, diamond start and chevron head). Edge statuses
  `ok / phantom / debt / accepted / design-error` -> `ok / violating (debt) / accepted`; there
  is no design contract, so no phantom edges. `deny` mark `✗` = a violating edge, `✓`
  = all its findings accepted.
- Data layers: `edges` (design + actual), `tests` (test-scope edges, off by default; a
  violating test edge is drawn when the deps layer is on), kind toggles `k-*`; finding groups
  `deps` (`deps/deny`, `deps/default`, `deps/cycle`), `cx` (`complexity/<metric>`), `stale`
  (`stale/file`, `bricks/unmapped`, other codes) with sub-layers; `deps` also has
  `deps/private`. Presets: Architecture =
  all edges plus the `deps` findings (violating edges are drawn red; complexity and stale hidden); Violations = no plain edges, all finding groups, error level only (a level chip
  overrides), scoped by `vs`; All = everything; Custom is set by any toggle.
- Findings = the diagnostics. Brick fill (`bd-red`/`bd-amber`) and the count badge show the
  open findings that pass the layers, in every preset (the prototype showed them in the
  Violations preset, and a complexity band otherwise; metric bands are not ported). Accepted
  findings are not counted.
- "info"-level findings (git history signals) do not exist. The prototype's per-finding review
  form (accept / fix-code / change-design / gap, expiry) is replaced by Accept (reason) and
  Unaccept.
- The prototype showed the source in the right panel and the tree in a bottom drawer; here a
  finding or function link opens the iframe popup with both side by side. Complexity tree
  horizontal ("ice") view, the DSM, the zone/area panels, project selector, `#hash` state (now
  the query string) and localStorage preset memory are not ported.
- Added, not in the prototype: shift+drag box selection on the background; findings list hover
  highlights the finding's edges and bricks (as in the prototype); keyboard `1 2 3`, `j/k`,
  `/`, `Esc`.
