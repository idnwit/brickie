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
- Every reader (`check`, `serve`, `interface`, `design`) uses one working design: the draft
  when one exists, else the published `brickie.toml` tree. Every design edit goes to the draft,
  versioned and based on the version the writer read; `design publish` makes the draft the
  published tree once the code matches its structure
  ([design/editor.md](design/editor.md#working-design)).
- Development stage: one current API and model path, no backward compatibility.

## Detail docs

| Doc | Summary |
| --- | --- |
| [design/measure.md](design/measure.md) | what `measure` reads, edge kinds, metrics, cache layout and lock |
| [design/check.md](design/check.md) | config tree, member selectors, rules, interfaces, findings, exit codes, `llms.txt` |
| [design/serve.md](design/serve.md) | webview decisions, brick score, layout port, embedded code viewer |
| [design/levels.md](design/levels.md) | nested bricks, exports, map-style zoom |
| [design/groups.md](design/groups.md) | peer brick groups within a level (shared rules, links and visual container) |
| [design/links.md](design/links.md) | decision record: allow-only rules and declared links |
| [design/editor.md](design/editor.md) | versioned design editing: draft, publish gate (`brickie design` and the viewer's Design mode) |
| [design/layout.md](design/layout.md) | crates, folders, crate boundaries |
| [design/complexity.md](design/complexity.md) | lib vs wiring flow, test placement, limits per tag |
| [design/roadmap.md](design/roadmap.md) | distribution, steps with status, open items, accepted behaviours |

## Documentation

Decision (two independent reviews): no separate spec and no requirement IDs. Each fact has one home,
and tests in `crates/cli/tests/docs.rs` compare the docs with the code.

| Fact | Home |
| --- | --- |
| Config keys, defaults, diagnostic codes with severity and fix | [llms.txt](llms.txt) (keys and codes are test-checked) |
| Rule and link semantics: reasons and edge cases | [design/links.md](design/links.md), [design/check.md](design/check.md) |
| UX method (principles P1..Pn, no widgets, colours or keys) | [design/ux.md](design/ux.md) |
| UI realisation (each section names the principles it serves) | [webui.md](webui.md) |

Reason: `llms.txt` must be complete and standalone anyway; a second normative copy only drifts.
