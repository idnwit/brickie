# Dependency edges

Which module uses which other module, lifted to bricks and checked against rules. Back to
[README.md](README.md). Facts: [../design/measure.md](../design/measure.md). Rules:
[../design/check.md](../design/check.md).

## Why it matters

Edges show the real structure of the code. Rules on edges stop a low-level brick from depending on
a high-level one, and cycle checks stop bricks from becoming one tangled unit.

## What is an edge

An edge goes from the module containing a reference (`from`) to the module that defines the item
(`to`). `to` is the defining module, taken from rust-analyzer (SCIP). A re-export is never the
target: if `a` re-exports `b::T` and `c` uses it through `a`, the edge is `c -> b`.

Each reference gives one edge of one kind.

| Kind | Where it appears |
| --- | --- |
| `uses` | any reference outside a `use` item, and the leaves of a private `use` |
| `reexports` | the leaves of a `use` with any `pub` visibility |
| `implements` | the trait path of `impl Trait for X`, when the trait is in another module |

## Scope

| Scope | Code |
| --- | --- |
| `test` | `#[cfg(test)]`, files under `tests/` and `benches/` |
| `prod` | everything else |

A `[[deps.rule]]` applies to `prod` by default. Cycles are checked on `prod` edges only.

## Bricks and rules

Bricks group modules in `brickie.toml`. An edge between two bricks is checked against the rules of
their lowest common parent. Edges inside one brick, or touching unmapped modules, are ignored.

```toml
[[deps.rule]]
from = "tag:app"
to = "tag:core"
kinds = ["uses", "implements"]   # default: all three
scope = "prod"                   # prod (default) | test | all
action = "allow"                 # allow | deny
```

The most specific rule wins (brick name 3, tag 2, `*` 1 per side, summed). Deny wins a tie. With
no match, `deps.default` decides. Codes: `deps/deny`, `deps/default`, and `deps/private` when a
brick uses another brick's item that its parent does not export.

## Cycles between bricks

Two or more sibling bricks that depend on each other, directly or through others, form a cycle.
The check runs per level (among the children of one composite) over `prod` edges of all kinds. One
`deps/cycle` error is reported per group of mutually dependent bricks. Config: `[deps] cycles =
"deny"` (or `"allow"`).

```rust
// brick `parse`            // brick `model`
use model::Node;            use parse::Token;   // model -> parse closes the cycle
```

Here `parse -> model` and `model -> parse` form a cycle. Fix: move the shared type to the lower
brick, or pass what the lower brick needs in through a parameter or trait.

## How to reduce problems

- Inverted edge: move the type to the brick both sides may depend on, or inject it.
- A cycle: break one edge as above. Do not merge bricks to hide it.

## Known limits

- The edge `to` follows the defining module, so a facade that only re-exports gets no edges for
  its users.
- Macro-generated references depend on what rust-analyzer resolves.
- Module-level cycles inside one brick are not reported.
