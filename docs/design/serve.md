# Serve

Scope: `brickie serve` webview, its decisions, the layout port, the embedded code viewer. Back to [../design.md](../design.md).

`brickie serve`: axum + sycamore, no htmx, no JS framework. It reads cache and configs like
`check` and shows the same `Diagnostic` values; every file:line span opens the code viewer.

| Decision | Reason |
| --- | --- |
| `crates/web/api` holds a typed `Endpoint` trait and every type crossing the port | a contract change fails to compile on both sides |
| One component tree, rendered on the server (SSR with hydration keys) and hydrated in wasm | the no-JS page is complete but read-only |
| Page JSON holds all diagnostics, levels and functions; nothing is filtered on the server | filters and presets are client-only |
| Layout runs once per level from the full edge set; presets and selection only change styles | no relayout, no flicker. The layout is not cached |
| View state lives in the URL query (`Params`), links keep real `href`s | back/forward and shared links render the same view |
| All writing endpoints pass one guard: loopback bind, loopback `Host`, same `Origin` | CSRF and DNS rebinding |
| Change detection by an mtime watcher feeding SSE (`/api/events`), not the `notify` crate | no new dependency, no inotify limits, same on every filesystem |
| New SSE connections get current state first | `EventSource` reconnect resyncs without replay |
| Measure job runs in the server process, single flight (`/api/measure`) | one cache writer; the page shows progress |
| Colours are CSS variables in `crates/web/ui/styles/`; classes change style, never geometry | state changes never move layout |

- Endpoint list: `crates/web/api`. Bundle: `scripts/build-ui.sh`. `crates/web/server/build.rs`
  embeds it, or serves the read-only SSR page with a cargo warning.
- **Brick score** (`crates/web/api/src/model/score.rs`, the only place): `open errors + 0.25 x
  open warnings + sum over functions of max(0, worst ratio - 1)`. Open means not accepted,
  whatever layers are shown.
- **Layers and presets**: Architecture = edges plus `deps` findings; Violations = error findings
  only; All; Custom. Finding groups: `deps`, `cx`, `stale`, `interface`.
- sycamore 0.9: `view!` treats any non-identifier expression as dynamic, so bind values to
  locals first. Element handlers need `web_sys` event types even on the server target.
- Zoom and level views: [levels.md](levels.md). Design mode: [editor.md](editor.md).

## Layout port

`crates/lib/algo/layout` is a function-by-function port of the Python prototype viewer's
JavaScript. Deviations: rows are tags, no areas, side columns or families; exact-integer LCG in
the annealer; one layout candidate; routes are cubic segments. Data remap: layers -> tags; edge
kinds -> `uses`, `implements`, `reexports`; statuses -> `ok`, violating, accepted.

| Decision | Reason |
| --- | --- |
| In edit mode the planned rule pairs are layout edges (after the graph edges) and drawn on their routes | lanes and bricks are ordered for the rules too; rule lines leave from side ports and go around boxes instead of cutting through them |
| The untagged lane goes where the edges' summed lane span is least (last on ties), inside the inbound/outbound lanes | it has no tag order to keep; in the middle it blocks the lanes it does not link |
| A row gap that no edge crosses is `GAPY_FREE` (36), not `GAPY` (72) | an empty gap needs room for the interface labels only |
| Planned edge marks ("?", "A") take the route point nearest the middle clear of boxes and other marks | marks where edges cross piled up on each other and on brick text |

## Code viewer

`/code/`: a generic read-only code browser (explorer, Monaco, marks, go to definition and
references). It knows nothing about bricks, rules or the cache.

- It is the composite `crates/lib/codeview` (tag `lib`, beside `algo`): `port` (contract: serde
  only, wasm-safe, `Provider` trait, endpoints, `postMessage` messages), `app` (wasm UI), `host`
  (axum router, embeds the `app` bundle as a file). `app` and `host` use only `port`.
  `web -> codeview` is allowed, `codeview -> *` denied.
- `crates/web/server/src/code/` implements `Provider` from config, cache and check result.
- The UI opens it in an iframe: `/code/?path=&line=&col=`. Marks are finding spans. Bands are
  one bar per function plus one per decision-tree node, heat = value / limit.
- `scripts/build-codeview.sh` fetches monaco-editor from npm (hash checked) into
  `assets/codeview/` (git-ignored). No CDN at runtime.

| Decision | Reason |
| --- | --- |
| iframe, not an inline component | keeps Monaco (3-5 MB) out of the UI bundle and its hydration |
| The port is the only contract | either side changes alone; `/code/` also runs standalone |
| `ToHost::Ready` message | messages sent on iframe `load` arrived before wasm started |
| Monaco fetched, not vendored | repo stays small |
| `POST` + JSON, paths checked by `valid_path` | no path escapes the workspace |

Open: hover (a `Hover` type, an endpoint, a `Provider` method).
