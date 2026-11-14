# Docs

- [usage.md](usage.md): install, commands, CI, building the viewer bundle.
- [design.md](design.md): overview and index of the design docs.
  - [design/measure.md](design/measure.md): measure facts and cache layout.
  - [design/check.md](design/check.md): config tree, rules, interfaces, findings.
  - [design/serve.md](design/serve.md): webview decisions, layout port, code viewer.
  - [design/levels.md](design/levels.md): nested bricks and zoom.
  - [design/editor.md](design/editor.md): versioned design editing (CLI and viewer).
  - [design/layout.md](design/layout.md): code layout and crate boundaries.
  - [design/complexity.md](design/complexity.md): complexity policy and limits.
  - [design/roadmap.md](design/roadmap.md): distribution, steps, open items.
- [metrics/README.md](metrics/README.md): plain explanation of each metric, with scored examples.
  - [cognitive](metrics/cognitive.md), [cyclomatic](metrics/cyclomatic.md), [nesting](metrics/nesting.md), [paths](metrics/npath.md), [params](metrics/params.md), [lines](metrics/lines.md), [dependencies](metrics/dependencies.md).
- [webui.md](webui.md): the reviewer webview, with screenshots in [screenshots/](screenshots/).
- [llms.txt](llms.txt): the guide for coding agents that `brickie llms.txt` prints (workflow, diagnostic codes and fixes, config schema, constraints).
- [examples/brickie.toml](examples/brickie.toml): an annotated example configuration.

Proposals (not yet implemented; implemented ones are folded into design.md):

- [proposals/vscode.md](proposals/vscode.md): VS Code extension; parked.
