# Complexity policy

Scope: how code is split into libs and wiring flows, test placement, limits. Back to [../design.md](../design.md). What each metric counts, with examples: [../metrics/README.md](../metrics/README.md).

Code is a **lib** or a **wiring flow**.

- A lib is logic with a simple, explicit contract: small pure functions or types, tested branch
  by branch. It may be internally complex.
- A wiring flow is linear orchestration with no decisions (cognitive about 8 or less). Decision
  logic found in a flow moves into a lib, with no new crate or dependency edge.

## Tests

- **Unit tests** sit in the same file under `#[cfg(test)] mod tests`, so they reach private
  items. Never widen visibility for tests. Use them only for simple-contract code.
- **E2e tests** go through a brick's public interface, in a separate file or `e2e_tests/`
  (goldens included). Complex orchestration is covered this way.
- Tests are exempt from the file-size guideline.

## Limits

All `error` unless noted; a nested file only tightens. Metrics: [measure.md](measure.md). Rule semantics: [check.md](check.md).

| Selector | cognitive | nesting | other | Config |
| --- | --- | --- | --- | --- |
| `tag:domain` | 15 | 4 | params 6, lines 80 | root `brickie.toml` |
| `tag:component` (codeview, UI) | 15 | 4 | | `crates/lib/brickie.toml` |
| `tag:algo` | 60 | 6 | | `crates/lib/brickie.toml` |
| `*` ceiling | 60 | 6 | lines 200 | root |
| `*`, test scope (`warn`) | 40 | | lines 150 | root |

A function over a limit is refactored unless a split clearly hurts. The UI has the same limits:
view-model decisions are pure functions with native tests. `lines` does not see inside `view!`.
