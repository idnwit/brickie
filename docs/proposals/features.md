# Proposal: Cargo features and targets

Status: phase 1 implemented; phase 2 proposed. Related: [../design/measure.md](../design/measure.md).

## Problem

`measure` ran `rust-analyzer scip` with default features on the host target. Code behind
`#[cfg(feature = "...")]` or `#[cfg(target_arch = "wasm32")]` was not indexed, so its edges were
missing: a rule they use looked unused. Frontend apps (wasm) hit this every time. Complexity is
unaffected (tree-sitter reads every `cfg` branch).

## Principles brickie promotes

| # | Principle | Reason |
| --- | --- | --- |
| P1 | Features are additive: the code builds with any combination | Cargo unifies features across the graph; exclusive features break other crates |
| P2 | Features select optional capabilities; `cfg(target_*)` selects the platform | wasm or native is a target, not a user choice |
| P3 | A feature adds an optional dependency or adapter; it does not change existing API | one shape per type |
| P4 | Conditional code lives at the edges: adapters and the composition root | the core stays independent of deployment |

## Phase 1: measure every feature and the declared targets

```toml
[measure]
additional_targets = ["wasm32-unknown-unknown"]   # the host is always measured
```

| Topic | Decision | Reason |
| --- | --- | --- |
| Features | always all features (rust-analyzer `cargo.features = "all"`); no setting | the design covers every code path; P1 makes it buildable |
| Targets | host always, plus `additional_targets` (default empty), in the root `brickie.toml` | normally only host and wasm; the name says it adds to the host |
| Runs | one scip run per target; edges are the union | rule checks see every edge |
| Hint | `cfg(target_arch = ...)` / `cfg(target_os = ...)` for a target not measured: `measure` prints one line suggesting `additional_targets` | never add a target silently; cost stays visible |
| Exclusive features | if the all-features run fails, the error says the features are likely not additive (P1) and how to fix it | P1 enforced by construction |

## Phase 2 (proposed, not started)

- Each edge carries `when` (`always`, `feature(x)`, `target(y)`), found by tree-sitter from the
  `#[cfg(...)]` around the reference and from `optional` dependencies.
- A rule may take `when` (`rule add app storage allow --when feature:storage`).
- `[cfg] allow_in = ["app", "adapters/*"]`: conditional code elsewhere is `cfg/placement`
  (warning) (P4).
- Viewer: a badge (`storage`, `wasm`) on conditional links.
