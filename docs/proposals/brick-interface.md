# Proposal: brick interface review

Status: **implemented** (2026-11-12); deviations and the edge-sites addition are listed at the
end. Current behaviour is in `docs/design.md` ("Cache layout", "Check", "View").

## Goal

A black-box view of every brick: what it declares public, what each item is (code + doc), and
who uses each item. For reviewers (viewer) and coding agents (CLI).

## Definitions

- **Declared interface** of brick B: the `pub` items (fn, struct, enum, trait, type, const,
  static, macro, method / associated item of a public type) defined in B's exported members,
  following the `exports` chain down (a composite's interface is the union of its exported
  children's interfaces). A leaf crate brick declares the items reachable from its crate root
  API; a leaf folder brick declares the `pub` items of its modules.
- **Uses** of an item: reference edges (from the cache) whose symbol is the item, from code
  outside B, plus references from test code anywhere (including B's own tests). An item that
  is only used inside B's non-test code is **unused**: it is public for nobody. A library item
  that nothing uses outside and no test exercises is untested surface.
- Uses are grouped by the using brick at B's level (its siblings), with file:line locations.

## Data

`brickie measure` records, per declared item, from SCIP `SymbolInformation` and the defining
occurrence: `symbol`, `kind`, display `name`, one-line `signature`, `doc` (first paragraph),
defining file and line. Cache schema bump. Use lists are computed at `check` / `view` time from
the existing reference edges (no extra cache).

## Diagnostic

`interface/unused` (warn): a declared item with no use. Subject is the brick, location is the
item definition. Accept via review as usual. Added to `codes.rs` and documented in `llms.txt`.

## Viewer

Brick details panel gets an **Interface** section (leaves and composites, also at the level
where the brick is a neighbour):

```
web/api · 12 items · 1 unused
▸ trait Endpoint      A server route: path + request/response        7 uses
    pub trait Endpoint { const PATH; type Req; type Res; }
    server  routes/review.rs:14
    ui      components/call.rs:9
▸ fn foo  ⚠ unused
```

- Sorted by use count, unused items marked and listed last; code and locations open the source
  popup.
- With an edge A→B selected, the details show the code behind it: every reference site from A
  into B (file:line, the written text, normal vs test, kind), grouped by the used item of B,
  each site opening the source popup. This works for allowed edges too (not only violating ones
  with findings) and for lifted / frame edges (the union of the underlying module edges).

## CLI

`brickie interface <brick-path> [--json] [--from <brick-path>]`: the same data as text (signature, doc, uses by
brick with locations, unused marked) or JSON, for coding agents. Documented in `llms.txt`.

`--from A` prints the sites of the edge A -> brick instead (by used item).

## Implementation notes and deviations

- **Declared, precisely.** Measure records every non-test `pub` / `pub(crate)` item with
  `visible_in` (the module subtree that can name it, from its visibility, its modules' visibility
  and `pub use` re-exports) and `exported` (public API of a library crate). A leaf declares an
  item when it is `exported` or its `visible_in` subtree reaches a module of another brick. So a
  `pub fn` in a private module of a folder brick is not interface, while a `pub(crate)` item of
  a folder brick that shares its crate is (the crate-internal interface between folders); a leaf
  that is a whole crate declares exactly the crate's public API.
- **Not items**: trait members (the trait is the item), trait-impl items, fields, enum variants,
  modules, `macro_rules!` (rust-analyzer gives them no `pub` signature) and anything in test code.
- **Members count as uses of their type**: a reference to a field, method, variant or impl item
  is a use of the type (and of the method item itself); `impl` segments and generic arguments
  are ignored when matching symbols. `pub use` re-exports are routes, not uses.
- **Unused per leaf**: `interface/unused` is reported once per leaf and item (uses from siblings
  count); composites show the same marker in the viewer relative to their own boundary.
- **Uses in the item's own module** are not recorded (the cache has no edges inside a module), so
  a `#[test]` in the same module as the item does not count; tests in a `mod tests` child do.
- **Signature** is rust-analyzer's one-line signature (`pub trait Endpoint`, `pub struct X`),
  without a trait's member list.
- **Endpoints are POST** (`/api/interface {brick}`, `/api/edge-uses {from, to}`), matching the
  UI's typed `call` wrapper; no narrowing filter on the interface list: the edge view replaces
  it with the full site list.
- **Viewer**: interface warnings are their own finding group (`interface`), off in the
  Architecture preset, and a fifth scorecard category.
