# brickie: design

Status: steps 1-4 implemented (`measure`, `check`, `view`, `llms.txt`, cleaned and restructured
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
     (no config needed)                 └──► brickie view   ──► webview   (both read brickie.toml)
```

- `measure` is the only step that reads code. It writes a cache with facts (crates, modules,
  files, edges, functions) and a content hash per source file; no brick data.
- `check` and `view` read the cache and the configs and resolve the brick tree on every run, so
  they are fast and need no rust-analyzer. Changing any `brickie.toml` (bricks or rules) never
  requires re-measuring.
- **Staleness**: `check`/`view` re-hash the files listed in the cache; changed, added or deleted
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
members = ["my-engine"]              # crate, crate::module or path:<dir>/
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

### 3. View: `brickie view [--config brickie.toml] [--port 7881] [--bind 127.0.0.1]`

Implemented; details in "View (implemented)" below.

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
3. `view`: **Implemented** (see "View (implemented)"). Shows the same `Diagnostic` values as
   `check`; every file:line span opens a source popup scrolled to and highlighting that line.
   The UX is a port of the Python prototype viewer (JS in one HTML file, not published): client-side Sugiyama layout, presets and data layers, selection
   scoping, source popup with the vertical complexity tree. Ported and dropped parts are listed in
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
   Phases 1-3 **implemented**: brick mapping out of the cache (schema 5), config tree with
   `path`/`path:`, partition checks, per-level deps and cycles via lifting, `exports` /
   `deps/private`, inherited complexity (see "Check"). Phase 4 **implemented**: the viewer shows one
   level at a time with map-style zoom (see "View", "Levels").

## Code layout (implemented)

A virtual workspace of five crates, grouped by layer (brickie checks this with the root
`brickie.toml` and `crates/{core,web}/brickie.toml`: cli -> web -> core, model is the base):

```
crates/core/model/   brickie-model: light, wasm-safe shared types: Level, codes (every
                     diagnostic code), Diagnostic values (Span, Note, RuleRef, DiagEdge),
                     FnTree/TreeNode
crates/core/engine/  brickie-core (library), src/ by concern:
  config/            schema (one level of the tree), selector, load (tree + include), validate,
                     member (member matching, specificity), resolve (module -> leaf path, partition)
  cache/             model (cache.json schema), files (tree walk), hash
  measure/           mod (run), metadata, rust_analyzer, lock, git, crates, modules, files,
                     edges, scip/ (SCIP -> rows, use scanner), complexity/ (tree-sitter metrics)
  rules/             lift (common ancestor, exports), deps, private, cycles, complexity, unmapped,
                     stale
  diag/              model (Finding -> Diagnostic), summary, text
  check/             config + cache + review -> diagnostics (shared by check and view)
  review/            brickie.review.toml read and edit
crates/web/server/   brickie-web: axum server (routes, render, data, tests); build.rs embeds
                     assets/ui/ when present
crates/web/ui/       brickie-ui: viewer (components, layout, state, model), compiled to wasm;
                     depends on brickie-model only, never on brickie-core
crates/cli/          brickie: the `brickie` binary (main.rs, cli/: clap args + one file per
                     command) and the integration tests (tests/check, tests/llms, tests/measure)
```

Source files stay small and focused (about 300 lines or fewer, tests aside).

## `brickie llms.txt` (implemented)

Prints `docs/llms.txt` (embedded with `include_str!`; no project or config is read). The guide is
the single place that tells coding agents how to read each diagnostic code and what they must not
do. `crates/core/model/src/codes.rs` is the list of codes; the test `guide_documents_every_diagnostic_code`
fails when a code is missing from the guide, so add a code there and in the guide together.

## Cache layout (implemented, schema_version 5)

`.brickie/cache.json` is written next to the config file. Top level: `schema_version`,
`tool_version`, `rust_analyzer`, `workspace_root`, `git` (`commit`, `dirty`; null outside git),
`crates`, `modules`, `files`. There is no brick data: `check` and `view` resolve bricks from
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
- `implements` edges come from the syntax (`impl Trait for X`), because rust-analyzer emits no
  SCIP relationships. Paths (NPath variant) add sequential decisions; see `crates/core/engine/src/measure/complexity/paths.rs`.
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
  id, a longer path or module id beats a shorter one; equal specificity across bricks is
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
- **Staleness** (`warn` `stale/file`; `rules/stale.rs`; config edits are never stale): candidates from
  `git diff --name-only --relative <commit>` and `git status --porcelain`, confirmed by sha256;
  without git or with a missing commit all cached files are hashed and crate source dirs walked
  for added files. A cache measured on a dirty tree also hashes all cached files (a file edited,
  measured, then reverted to HEAD is invisible to git). Added files count only under a crate
  root's directory. Uncompiled files are in the cache (`compiled: false`), so they are compared
  like any other and not reported as added.
- **Findings** are `Diagnostic` values (`diag/model.rs`): `id` (first 8 hex of sha256 of
  `code\0subject`), `level`, `code` (`deps/deny`, `deps/default`, `deps/private`, `deps/cycle`,
  `complexity/<metric>`, `stale/file`, `bricks/unmapped`), `brick_level`, `message`, `span`
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

## View (implemented)

Stack: axum 0.8 + sycamore 0.9, no htmx and no JS framework. Code layout (small focused files,
folders by concern):

- `crates/web/server/src/`: `lib.rs` (options, state, `run`), `routes/` (`index`, `source`, `api`, `review`,
  `assets`), `render/` (page and source-popup HTML shells), `data/` (page data from the cache:
  `diagnostics`, `graph/` (per-level aggregation: `edges`, `level`, `heat`), `functions`,
  `time`), `tests/`.
- `crates/web/ui/` (`brickie-ui`): `model/` (`params`, `diag`, `graph`, `levels`, `function`), `layout/`
  (`rank`, `order`, `anneal`, `place`, `ports`, `compute`, `geometry`, `path`, `smart`,
  `measure`), `state/` (`layers`, `selection`, `focus`, `eval`, `shared`, `ctx`), `components/`
  (`app`, `keys`, `canvas/` incl. `frame`, `panels/` incl. `node`, `controls/` incl. `crumbs`,
  `source/`), `platform.rs`, `wasm.rs`, `styles/*.css`.

- **One component tree, two runtimes.** The server renders `brickie_ui::app` with
  `sycamore::render_to_string` (output carries `data-hk` hydration keys) into `<div id="app">`
  and embeds the page data as JSON in `<script id="brickie-data">`. The wasm bundle reads that
  JSON, rebuilds the same tree and hydrates `#app`. The page JSON holds all diagnostics, every level of the
  brick tree (bricks, tag rows, aggregated edges per level, see "Levels") and the functions;
  nothing is filtered on the server.
- **Layout runs on the client, once per level.** `crates/web/ui/src/layout/` is a pure deterministic
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
  (`1`: findings of this level and below only). Every link has the real `href`; hydrated, a plain click
  updates the signal and `history.pushState`s, `popstate` re-reads the URL. A full load with
  params renders the same view.
- **Without wasm** (bundle not built) the SSR page is complete but read-only: the canvas, filters
  and links are server rendered; pan/zoom/drag, dropdowns and accept/unaccept need the bundle.
- **Endpoints**: `GET /` (page), `GET /source?file&line&col&end_col`, `GET /api/diagnostics`,
  `/api/functions` (`brick=` takes tree paths such as `root/web`, matching the subtree),
  `/api/graph` (the levels, JSON, same params), `POST /review` (JSON
  `{id, reason}`; `reason: null` unaccepts), `/static/ui/brickie_ui{.js,_bg.wasm}`. Cache and
  config are read per request (re-run `measure`, refresh).
- **Diagnostics** are `check`'s `Diagnostic` values field for field (`DiagView`), plus a snippet
  with the span marked and `bricks`. A selected finding (`item=`) shows the same card as before
  (code, message, spans as source links, rule, help, notes, Accept/Unaccept).
- **Source popup**: a modal with an iframe of `/source`. The page lists the file in fixed 18 px
  rows with line anchors `#L<n>`, marks the target line or span, and, for the innermost function
  around the target line, draws the vertical complexity tree in a sticky gutter at the left
  (`complexity::fn_tree`, tree-sitter, computed on demand for each request, nothing is cached):
  one column per nesting depth, one bar per decision branch over the lines it spans, opacity
  proportional to log(paths), cold to hot colour by depth, code lines tinted by their deepest
  branch; clicking a bar marks its lines. Only files listed in the cache are served; syntax that
  could traverse (`..`, `.`, empty segments, absolute, `\`, NUL) is 400, unlisted files,
  non-regular files (symlinks), files over 4 MB and paths resolving outside the cached workspace
  root are 404.
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
- **Brick score** (`crates/web/ui/src/model/score.rs`, the one place scores are computed): `score = open
  errors + 0.25 x open warnings + sum over functions of max(0, worst ratio - 1)`. A finding is
  open when its level is `error`/`warn` and it is not accepted, whatever layers are shown. A
  function's ratio is its worst `value / limit` over the metrics that have a limit under the rules
  that apply to its brick and scope (`rules::complexity_limits`, the same lookup as `check`).
  The server embeds the static part in each `BrickView` (`worst_ratio`, `hot`: functions over a
  limit, worst first); the page adds the open counts (`brick_scores`), so scores follow
  accept/unaccept. The score appears as a corner badge (`3E 1W`, or a green check when clean), a
  heat bar under the brick name (length = ratio / 2, full at 2x the limit; colour by level: below
  0.8, 0.8-1, 1-1.5, over 1.5), the brick tooltip, the "bricks by tag" list (worst first inside
  each tag, ordered by the score at load) and the scorecard in the details panel (open findings
  per deps / cycle / complexity / stale, the worst 3 functions with metric, value/limit and ratio
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
- **Known limits**: measured time is the cache file's mtime; accept needs the wasm bundle; the
  popup has no syntax colouring; the layout is not cached (about 25 ms natively for 15 bricks
  and 33 edges in release; the SSR page recomputes it per request).
- Tests: `crates/web/server/src/tests/` (tower `oneshot`: index, layers and presets as params, JSON
  endpoints, source tree/highlight/escaping/traversal/symlink, review round trip, missing
  cache); `crates/web/ui/src/layout/tests.rs` (no edge crosses a box, boxes do not overlap, determinism,
  crossings at most the config order, rows follow tags, empty graphs, re-routing after a drag);
  `crates/web/ui/src/state/` (layers round trip through the URL, presets, selection,
  scoping); `crates/core/engine/src/measure/complexity/tests.rs`; `components/source/mod.rs` (gutter rendering).

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
- **Navigation**: double-click on a composite zooms in (the only way in: no key, button or
  panel link), on a neighbour opens its enclosing level; Backspace zooms out, Esc closes the popup, then the finding, then the
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

### Layout: original JS function -> Rust (`crates/web/ui/src/layout/`)

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
| `vertHTML`, `iceCol`, `heat` | `components/source/tree.rs` (`ice_col`, `heat`, `bar`) | whole file listed, tree of one function; no horizontal "ice" tab |
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
