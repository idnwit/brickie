# Metrics

Plain explanations of what brickie measures. Back to [../README.md](../README.md).

`brickie measure` reads the code and records numbers. `brickie check` compares them with the rules
in `brickie.toml` and reports findings. Function metrics come from the syntax tree
(tree-sitter-rust). Dependency edges come from rust-analyzer.

| Metric | Question it answers | Limit in this repo | Page |
| --- | --- | --- | --- |
| `cognitive` | How hard is the function to read? | 15 (`tag:domain`), 60 ceiling | [cognitive.md](cognitive.md) |
| `cyclomatic` | How many independent paths need a test? | none set | [cyclomatic.md](cyclomatic.md) |
| `nesting` | How deep do branches and loops stack? | 4 (`tag:domain`), 6 ceiling | [nesting.md](nesting.md) |
| `paths` | How many ways can execution run through the function? | none set | [npath.md](npath.md) |
| `params` | How many arguments does the signature take? | 6 (`tag:domain`) | [params.md](params.md) |
| `lines` | How long is the function? | 80 (`tag:domain`), 200 ceiling | [lines.md](lines.md) |
| dependency edges | Which brick uses which, and is there a cycle? | `[deps]` rules | [dependencies.md](dependencies.md) |

No metric has a built-in default limit. A metric without a limit in any `[[complexity]]` rule is
measured but never checked. The limits above are this repo's own, from the root `brickie.toml`
(policy: [../design/complexity.md](../design/complexity.md)).

## In `brickie.toml`

```toml
[[complexity]]
bricks = "tag:domain"   # selector, default *
cognitive = 15          # any of cognitive, cyclomatic, nesting, paths, params, lines
nesting = 4
scope = "prod"          # prod (default) | test | all
level = "error"         # error (fails check) | warn
```

- A limit is inclusive: `cognitive = 15` accepts 15 and fails at 16.
- Per function and metric the most specific rule applies. A nested `brickie.toml` can only
  tighten a limit.
- `level = "error"` makes `brickie check` exit 1. `warn` is shown but does not fail.
- Functions in test code (`#[cfg(test)]`, `#[test]`, `tests/`) are checked only when `scope` is
  `test` or `all`.

## In diagnostics

Each function metric over its limit gives one finding per function with the code
`complexity/<metric>`, for example `complexity/cognitive`. Dependency problems give `deps/deny`,
`deps/default`, `deps/private` and `deps/cycle`. How to fix each one is in
[../llms.txt](../llms.txt).

## Shared rules for function metrics

- Each `fn` with a body is measured on its own. A nested `fn` is a separate function and adds
  nothing to the outer one.
- Macro bodies are not parsed. Branches inside `println!(..)`, `vec![..]` or `view! {..}` are
  invisible to every metric except `lines`.
- Closures add nesting but no cost of their own.
- `if let`, `while let` and `match` guards use the same rules as the plain forms.
