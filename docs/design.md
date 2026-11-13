# brickie: design

Overview and index. Decisions, invariants and roadmap. Behaviour detail lives in the code,
`brickie <cmd> --help`, [llms.txt](llms.txt) (config schema, diagnostic codes, fixes) and
[../README.md](../README.md).

## Goal

One Rust binary, `brickie`, with two users:

1. **CI**: measure a workspace, apply the project's rules, exit non-zero on violations.
2. **Reviewer webview**: see bricks, edges, violations and function complexity next to the source,
   to review what coding agents changed.

The tool has **no opinion about architecture**: no built-in layers or zones. Rules live in the
user's `brickie.toml`. A preset is a rules file other configs include.

## Architecture

```
brickie measure --> .brickie/cache.json --> brickie check  --> findings (text/json, exit code)
  (no config)                           \-> brickie serve  --> webview   (both read brickie.toml)
```

## Principles

- `measure` is the only step that reads code. The cache holds facts and file hashes, no brick data.
- `check` and `serve` resolve the brick tree from cache plus configs on every run. They need no
  rust-analyzer. Editing a `brickie.toml` never needs a re-measure.
- Bricks nest. An outer level decides what inner parts may use; a brick exports only what
  outsiders may use.
- One `Diagnostic` model for `check` and the webview. Review decisions survive re-runs.
- Crates exist only at real boundaries; complexity limits only tighten down the tree.
- One engine for preview and publish in the editor; `brickie.toml` stays the source of truth.
- Development stage: one current API and model path, no backward compatibility.

## Detail docs

| Doc | Summary |
| --- | --- |
| [design/measure.md](design/measure.md) | what `measure` reads, edge kinds, metrics, cache layout and lock |
| [design/check.md](design/check.md) | config tree, member selectors, rules, interfaces, findings, exit codes, `llms.txt` |
| [design/serve.md](design/serve.md) | webview decisions, brick score, layout port, embedded code viewer |
| [design/levels.md](design/levels.md) | nested bricks, exports, map-style zoom |
| [design/editor.md](design/editor.md) | design editor, change sessions, UX principles |
| [design/layout.md](design/layout.md) | crates, folders, crate boundaries |
| [design/complexity.md](design/complexity.md) | lib vs wiring flow, test placement, limits per tag |
| [design/roadmap.md](design/roadmap.md) | distribution, steps with status, open items, accepted behaviours |
