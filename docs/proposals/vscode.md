# Proposal: VS Code extension, one UI for web and editor

Status: **parked** (2026-11-12): the user chose to focus on the web viewer first. Not implemented.

## Goal

The brickie viewer (brick map, details, interface, code behind edges) runs unchanged in the
browser and inside VS Code. What differs between the two is only the **host**: how code is
opened, how navigation is kept, how the theme is chosen. That difference goes behind a port.

## Host port (UI side)

Today `brickie-ui`'s `platform` mixes browser calls (fetch, history, popup) with decisions
(a code link opens the `/code/` popup). Split it:

- `web/api` gains the contract: `UiToHost` / `HostToUi` message enums (serde), next to the
  endpoints.
  - `UiToHost`: `Ready`, `OpenCode { path, line, col }`, `OpenBrickCode { brick }`,
    `OpenExternal { url }`, `Navigated { url }` (params changed).
  - `HostToUi`: `Navigate { url }` (go to params), `Reveal { path }` (select the brick that owns
    a file), `Theme { dark }`, `Refresh` (cache re-measured).
- `ui/platform` exposes one `Host` with two implementations, picked at start-up:
  - `WebHost` (default): `OpenCode` opens the code viewer popup (`/code/`), `Navigated` pushes
    browser history, theme from `prefers-color-scheme`.
  - `EmbeddedHost` (URL has `host=vscode`, i.e. the page runs in an iframe of a VS Code
    webview): every `UiToHost` is posted to `window.parent`; `HostToUi` arrives as messages.
- Components call only `Host` (`host.open_code(..)`), never `window` or the popup directly; the
  brick check keeps `components` off the browser APIs as today.

Data stays HTTP (the `web/api` endpoints) in both cases.

## VS Code extension (`editors/vscode`, TypeScript, outside the Cargo workspace)

- Starts `brickie view --port 0 --bind 127.0.0.1` for the workspace folder (needs a
  `--print-port` or a ready line to read the port) and stops it with the window.
- Command **brickie: Open map**: a webview panel whose page is a thin shell with one iframe to
  `http://127.0.0.1:<port>/?host=vscode` (CSP allows only that origin). The shell relays messages
  between the iframe and the extension.
- Handles `OpenCode` with `vscode.window.showTextDocument` at the line/column (the real editor,
  with rust-analyzer); `OpenExternal` with `env.openExternal`.
- **Follow editor** (toggle): on active editor change, sends `Reveal { path }`; the map selects
  that file's brick and zooms to its level.
- Commands **brickie: Measure** / **Check** run the CLI in a terminal/task and send `Refresh`.
- Theme: sends `Theme { dark }` from the VS Code color theme kind.

In VS Code the code viewer (`codeview`) is not used: code opens in the editor. Its port stays
for the browser.

## Bricks

- `web/api`: + host messages (contract).
- `web/ui/platform`: `Host`, `WebHost`, `EmbeddedHost`; `components` use `Host` only.
- `editors/vscode`: not a Rust crate; outside brickie's checked tree (documented in design.md).
- `cli`: `brickie view --port 0` prints the bound address on a machine-readable line.

## Later (separate proposal)

`brickie lsp` (diagnostics, hover, code lens for complexity, accept action) used by the same
extension and by Zed; a Bricks tree view in the side bar.

## Open questions

1. Data over HTTP from a local server (simple, reuses everything) vs a fully bundled webview
   with data over `postMessage` (no server, stricter CSP, more work). Proposed: HTTP first.
2. Publish to the marketplace, or local `.vsix` only for now? Proposed: `.vsix`.
