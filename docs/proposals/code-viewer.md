# Proposal: code viewer (explorer + Monaco) as an independent component

Status: **implemented** (2026-11-12), first version: explorer, tabs, Monaco, marks, definition
and references; hover later. See "Implemented" at the end for the deviations.

## Goal

Read the code of a brick like in VS Code: a file explorer, tabs, the Monaco editor (read-only),
go to definition / references, findings in the code. The viewer is generic: it knows nothing
about bricks, rules or the cache. brickie feeds it through a **port**; any other host could too.

## Bricks first

New composite `crates/lib/codeview` (under `lib`, a sibling of `algo`): independent business, no brickie
types, its own complexity class.

```toml
# brickie.toml (root): added
[[brick]]
name = "codeview"
path = "crates/lib/codeview"            # composite: port, app, host
tags = ["lib"]
desc = "Generic code viewer: file explorer, tabs and Monaco, fed by a host through a port"

[[deps.rule]]
from = "web"
to = "codeview"
scope = "all"
action = "allow"
reason = "the server mounts the code viewer and implements its provider port"
# codeview -> anything: denied by default (it must stay independent)
```

```toml
# crates/lib/codeview/brickie.toml
# The code viewer. `port` is the contract; `app` (wasm UI) and `host` (axum router + the
# embedded bundle) build on it only. A host application depends on `port` and `host`.
exports = ["port", "host"]

[[brick]]
name = "port"
members = ["codeview-port"]
desc = "The contract: data types, the provider trait, HTTP endpoints and iframe messages"

[[brick]]
name = "app"
members = ["codeview-app"]
desc = "The viewer UI (Sycamore, wasm): explorer, tabs, Monaco glue, navigation"

[[brick]]
name = "host"
members = ["codeview-host"]
desc = "axum router over a provider: serves the bundle and the port endpoints"

[deps]
default = "deny"
cycles = "deny"

[[deps.rule]]
from = "app"
to = "port"
scope = "all"
action = "allow"
reason = "the UI speaks the port's endpoints and messages"

[[deps.rule]]
from = "host"
to = "port"
scope = "all"
action = "allow"
reason = "the router serves the port's endpoints from a provider"
```

`host` embeds the `app` bundle as a file at build time (not a Rust dependency), so `host` and
`app` stay independent. Monaco is vendored under `crates/lib/codeview/app/vendor/` and embedded the
same way (offline, no CDN).

## Port (`codeview-port`, wasm-safe, serde only)

Data:

- `Tree { roots: Vec<Node> }`, `Node { id, label, path: Option<String>, badge: Option<Badge>,
  children, kind: NodeKind, tags: Vec<String> }`. `kind` (`Group`, `Brick`, `Composite`, `Crate`,
  `Folder`, `File`) and `tags` are presentation only: the viewer draws a marker and colour per
  kind and each tag as a chip with a hue derived from its text (both default when absent). A host may give several named trees (`views`); brickie gives "Bricks" and
  "Folders". The viewer only renders labelled trees.
- `FileText { path, text, lang, stale: bool }`.
- `Mark { span, level: info|warn|error, message, link: Option<String> }`: findings, highlights.
- `Band { start_line, end_line, depth, heat: 0..=1, label, detail }`: nested line-range
  annotations drawn as bars in the gutter (brickie: each function and its decision-tree nodes;
  hover shows label and detail, click highlights the lines, a toggle hides them).
- `Hover { span, markdown }`, `Location { path, span, label }` (label: e.g. the using brick).

Provider trait, implemented by the host application (server side):

```rust
pub trait Provider: Send + Sync + 'static {
    fn views(&self) -> Vec<(String, Tree)>;
    fn file(&self, path: &str) -> Option<FileText>;
    fn marks(&self, path: &str) -> Vec<Mark>;
    fn bands(&self, path: &str) -> Vec<Band>;
    fn hover(&self, path: &str, pos: Pos) -> Option<Hover>;
    fn definition(&self, path: &str, pos: Pos) -> Vec<Location>;
    fn references(&self, path: &str, pos: Pos) -> Vec<Location>;
}
```

Endpoints (same `Endpoint { PATH, Req, Res }` pattern as `web/api`, own trait, no brickie
dependency): `Views`, `File`, `Marks`, `Bands`, `Hover`, `Definition`, `References`.

Iframe messages (`postMessage`, JSON, both directions):

- host page → viewer: `Open { path, span? }`, `Filter { view, node }` (scope the explorer, e.g.
  to one brick), `Theme { dark }`.
- viewer → host page: `Opened { path }`, `Link { href }` (a mark's link clicked, e.g. a
  finding), `Close`.

## Embedding: iframe

The brickie viewer embeds the code viewer as `<iframe src="/code/?path=..&line=..">`, mounted
by `codeview_host::router(provider)` under `/code`. Reasons:

- Isolation: Monaco's 3-5 MB and its JS stay out of the brickie UI bundle and its hydration;
  it loads only when code is opened.
- The contract is only the port (HTTP + messages); either side changes independently.
- Works standalone too: `/code/` in its own tab is a full code browser.

The source popup becomes this iframe. The old server-rendered `/source` page is removed (its
complexity tree became bands); without JavaScript a link opens `/code/` in a tab, which needs
JavaScript too.

## brickie side (adapter)

`web/server/src/code/` (folder brick `code`, → `data`) implements `Provider` from the cache:

- views: "Bricks" (level → brick (kind `Brick` / `Composite`, config tags as chips) → crate →
  folders → files, like "Folders"; badges = open E/W) and "Folders";
- file: from disk, `stale` when the hash differs from the measured one (navigation may be off);
- marks: findings of the file;
- bands: every function of the file with its complexity tree (implemented);
- hover: interface signature + doc of the symbol under the cursor;
- definition / references: from the cached reference edges (references labelled with the
  using brick, cross-brick ones first).

The brickie UI sends `Open` / `Filter` when a site, finding or brick is clicked, and handles
`Link` by selecting the finding.

## Viewer UX

```
┌ Explorer [Bricks|Folders] ┬ level.rs × | call.rs × ───────────── ┐
│ ▾ web/api          5W     │  Monaco, read-only, theme follows   │
│   ▾ src/model             │  marks: squiggles + gutter          │
│      graph.rs      ●2     │  F12 definition, Shift+F12 refs     │
│ ▸ web/ui          27W     │  hover: signature + doc             │
└───────────────────────────┴─────────────────────────────────────┘
```

Ctrl+P quick open, Ctrl+F find (Monaco), tabs, explorer filtered when opened from a brick
("show all" clears it). A URL with `filter` and no `path` (`/code/?filter=<node>&view=Bricks`)
opens the filtered node's entry file (`lib.rs` / `main.rs` / `mod.rs` at the shallowest depth, else
the first file). brickie's brick details panel has an "Open code" link to such a URL (popup, or a
plain link without JavaScript).

## Open questions

1. Name: `codeview` ok?
2. Complexity class of `tag:lib`: same as `algo` (cognitive 60, warn) or domain limits?
3. First version scope: explorer + tabs + Monaco + marks + definition/references; hover later?

## Implemented

Answers to the open questions: the name is `codeview`; `tag:lib` has domain-like limits
(cognitive 15, nesting 4, warn); the first version leaves hover out. Deviations from the text
above:

- No `Hover` type, endpoint or provider method yet; no complexity info marks.
- Endpoints are `POST` with JSON bodies at `api/views`, `api/file`, `api/marks`,
  `api/definition`, `api/references`, relative to the mount (`/code/api/...`); the host checks
  paths (`valid_path`) before a provider sees them.
- `Node` carries an `id` (the target of `Filter`); brickie uses the brick's level path
  (`root/web/server`) for brick nodes.
- `ToHost::Ready`: the viewer says when it listens, and the host page sends `Filter` / `Open`
  then (messages sent on the iframe's `load` arrived before the wasm started).
- Monaco is not vendored in git: `scripts/build-codeview.sh` fetches a pinned, integrity-checked
  npm tarball into the git-ignored `assets/codeview/` with the app bundle, and `codeview-host`
  embeds that directory (see docs/design.md, "View").
