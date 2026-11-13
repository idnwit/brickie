# Proposal: VS Code extension, one UI for web and editor

Status: **parked** (2026-11-12): the web viewer comes first. Not implemented.

## Idea

The viewer runs unchanged in the browser and in VS Code; only the **host** differs (how code
opens, how navigation and theme are kept). That difference goes behind a port.

## Design

- **Host port**: `web/api` gains `UiToHost` (`Ready`, `OpenCode {path, line, col}`,
  `OpenBrickCode`, `OpenExternal`, `Navigated`) and `HostToUi` (`Navigate`, `Reveal {path}`,
  `Theme`, `Refresh`). `ui/platform` exposes one `Host`: `WebHost` (default: code popup, browser
  history) or `EmbeddedHost` (URL `host=vscode`: messages to `window.parent`). Components call
  only `Host`. Data stays HTTP in both.
- **Extension** (`editors/vscode`, TypeScript, outside the Cargo workspace): runs
  `brickie serve --port 0 --bind 127.0.0.1` (needs a machine-readable bound-address line);
  command "Open map" shows a webview with one iframe to the server (CSP allows only that
  origin); `OpenCode` opens the real editor via `showTextDocument`; "Follow editor" sends
  `Reveal`; Measure / Check commands send `Refresh`; theme from the VS Code theme kind.
- `codeview` is not used in VS Code (code opens in the editor); its port stays for the browser.

## Open

1. HTTP from a local server (simple, reuses everything) vs a fully bundled webview over
   `postMessage` (no server, stricter CSP, more work). Proposed: HTTP first.
2. Marketplace or local `.vsix`. Proposed: `.vsix`.
3. Later, separate proposal: `brickie lsp` (diagnostics, hover, complexity lens, accept action)
   for the same extension and Zed; a Bricks tree view.
