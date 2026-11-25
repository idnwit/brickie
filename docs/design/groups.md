# Groups

Scope: peer brick groups within a level. Back to [../design.md](../design.md).

Status: **Implemented** (core check, layout, design editor/CLI, web API/server/UI, and [../llms.txt](../llms.txt)).

## Goal

A `[[group]]` is a config-level grouping of peer bricks that share the same architectural role at
one level (for example `services` = `auth`, `billing`, `notify`). Grouped bricks share one contract
of `[[rule]]` (allow-only architectural principles), `[[link]]` (explicitly declared dependencies
permitted by rules; see [links.md](links.md)), `tags`, `exports`, `[[complexity]]`, and cycle
detection. The viewer draws edges to the group container instead of individual member bricks,
collapsing N×M per-member edges and repeated `[[link]]` entries into edges to the group container
without creating a composite folder or changing brick paths.

| Aspect | Composite (`path = "..."`) | Group (`[[group]]`) |
| --- | --- | --- |
| Purpose | Encapsulated subsystem of different roles (`web` = `api` + `server` + `ui`) | Config and visual grouping of homogeneous peers (`services` = `auth`, `billing`, `notify`) |
| Config file | Separate `<path>/brickie.toml` | Inline in the same `brickie.toml` |
| Brick path | Nests path (`web/api`) | Unchanged (`auth` stays `auth`) |
| Visibility | Private by default (`exports = []`) | No own `exports`; members reached directly through the group (composite members still enforce their own `exports`) |
| Internal edges | Allowed via the composite's own rules and links | Forbidden between members of the same group |
| Viewer | Opaque box; zoom in to see children | Container box showing member bricks inside on the same canvas |

## Proposed rule (for [../llms.txt](../llms.txt))

Add Rule 10 to the "Rules" section of [../llms.txt](../llms.txt) and clarify Rules 4–9 (Edges,
Rules, Links, Exports, Independent, Complexity) so each states that a group acts as one unit at its
level:

> 10. **Groups**: a `[[group]]` (`name`, optional `tags`, `desc`) groups sibling `[[brick]]`s of
>     the same level whose `group` field names it. Group and brick names share one namespace in the
>     level. At its level, a group acts as one unit for `[[rule]]`, `[[link]]`, `tags`, `exports`
>     (of the enclosing composite), `[[complexity]]`, and `deps/cycle`: target the group name (or
>     `tag:<t>` matching its tags), never a member brick or `group/member`. Member bricks keep
>     their own brick path (grouping does not nest paths), cannot set non-empty `tags` or
>     `independent = true`, and cannot depend on each other in any scope (`prod` or `test`;
>     `deps/unlinked`). Because `deps/cycle` runs over units at the level, edges such as
>     `auth -> bus` and `bus -> billing` form a `services <-> bus` cycle. A member brick may be a
>     composite (`path = "<dir>"`): outside edges enter the group transparently and still obey that
>     composite's `exports`, while links target `group` (never `group/member/child`). Reason: peers
>     with the same role share one contract and one visual box without a separate folder, path
>     change, or facade.

Trade-off accepted by design: collapsing per-member links into group-level links trades per-member
outbound link granularity (a link `services -> db` covers every member of `services`) for concise
configuration and collapsed canvas edges. When members need distinct outbound permissions, split
them into separate groups or leave exceptions ungrouped.

## Schema

`brickie.toml` syntax:

```toml
[[group]]
name = "services"              # unique among bricks and groups in this level
tags = ["domain"]              # optional; applies to the whole group
desc = "Domain services"       # optional one-liner shown in the viewer

[[brick]]
name = "auth"
group = "services"             # optional; belongs to at most one group of this level
members = ["acme-auth"]

[[brick]]
name = "billing"
group = "services"
members = ["acme-billing"]

[[brick]]
name = "api"
members = ["acme-api"]

[[rule]]                       # architectural principle allowing api -> services
from = "api"
to = "services"
reason = "api calls domain services"

[[link]]                       # declared dependency allowed by the rule above
from = "api"
to = "services"
reason = "handlers dispatch to services"
```

- `[[group]]` has no `independent` field: by Rule 8 ([links.md](links.md#independent-bricks)),
  inbound edges to independent bricks already require no rule or link and are hidden in the viewer,
  so the N×M crossing problem does not apply.
- `[[group]]` lists concatenate across `include`d files like `brick`, `rule`, `link`, and
  `complexity`. Because includes merge before validation, a `[[brick]]` in an included file may
  reference a `[[group]]` in the including file (and vice versa).

## Validation (`config/invalid`)

| Invalid config | Error |
| --- | --- |
| Group name is empty, has whitespace, or contains `/`, `*`, `root`, or `tag:` | invalid group name |
| Group name duplicates a brick or another group in the level | duplicate name |
| `brick.group` names a group not declared in the level | unknown group |
| Grouped brick sets non-empty `tags` (empty `tags = []` is ignored as default) | tags must be set on the group |
| Grouped brick sets `independent = true` (`independent = false` is ignored as default) | grouped bricks cannot be independent |
| `[[rule]]`, `[[link]]` (`from` or `to`), `exports`, or `[[complexity]]` names a grouped brick or `group/member` | target the group name instead |

Known tags collected during validation include tags declared on both `[[brick]]` and `[[group]]`
entries so `tag:<t>` selectors referencing group tags pass validation.

## CLI (`brickie design`)

Rename the existing composite-creation commands to avoid colliding with peer groups (breaking
change across `crates/core/model`, `crates/core/change`, `crates/cli`, `crates/web/ui`,
[../llms.txt](../llms.txt), [editor.md](editor.md), and [../webui.md](../webui.md); existing
`.brickie/history/log.jsonl` entries written with old op tags are not migrated during development):

- Composite creation:
  - `brickie design brick add NAME --kind composite ...` (was `--kind group`)
  - `brickie design brick nest NAME CHILD...` / `brickie design brick unnest PATH` (was `group` /
    `ungroup`)
- Peer groups (CLI paths omit the `root/` prefix, e.g. `services` or `core/services`, matching
  `brick` commands; `--in P` names an enclosing composite level, never a group):
  - `brickie design group add NAME [--in P] [--tags a,b] [--desc D]`
  - `brickie design group set PATH [--tags a,b] [--desc D]`
  - `brickie design group rename PATH NEW`
  - `brickie design group rm PATH`
  - `brickie design brick add NAME [--group NAME] ...`
  - `brickie design brick set PATH --group NAME` (empty `--group ""` ungroups the brick)

### Operation lifecycle

| Operation | Behaviour |
| --- | --- |
| `brickie design group rm PATH` | Fails (`design/invalid`) if any brick still references the group or if any `rule`/`link`/`exports`/`complexity` still names it (including parent `composite/group` links) |
| `brickie design group rename PATH NEW` | Rewrites `brick.group` on all member bricks in the level (including across `include`d files), local `rule`/`link`/`exports`/`complexity` references, and parent `composite/group` links |
| `brickie design brick set PATH --group NAME` | Fails (`design/invalid`) if the brick has non-empty `tags`, `independent = true`, or direct `rule`/`link`/`exports`/`complexity` references (no silent stripping) |
| `brickie design brick move PATH --into DEST` | Refuses `--into <group>` (`design/invalid`); refuses moving a grouped brick (ungroup first); refuses moving into a grouped composite if `rename_refs` would create direct refs to that grouped brick |
| `brickie design brick nest` / `unnest` | `nest` refuses grouped bricks (ungroup first); `unnest` on a grouped composite lifts its children as ungrouped bricks; `unnest` on a composite that contains `[[group]]`s lifts those groups into the parent (failing on name collisions) |
| `brickie design apply` | A later line may reference a group added on an earlier line in `--group`, `rule`, `link`, `export`, or `limit` (matching `brick add` forward references) |

`brickie design show` (text and JSON) lists each level's `groups: [{name, tags, desc, members}]`
and each brick's optional `group` field.

## Viewer

- **Canvas**: each group renders as a single labelled container node holding its member bricks as
  clickable cards on the same canvas (an empty 0-member group renders with a compact minimum box
  and a "drop bricks here" placeholder). All incoming and outgoing edges connect to the group
  border.
- **Intra-group violations**: forbidden edges between two members of the same group are attached to
  the group node via a dedicated `internal_edges` list (never dropped as self-edges and never
  double-counted in `EdgeView` fan-in/fan-out scores). The container displays both a violation
  count badge in its header and internal red connectors between the offending member cards;
  findings appear in the inspector when selecting the group or either member.
- **Selection and zoom**: selecting the group scopes the inspector to the group and its members;
  selecting a member card scopes findings, functions, and `/docs/<brick>` to that member. Groups
  have no zoom level of their own; a member brick that is a composite keeps its zoom affordance.
- **Design mode**: dragging an ungrouped brick onto a group box (empty or populated) sets
  `brick.group`; dragging a member card out of the box clears `brick.group`. When the dragged brick
  has existing `tags` or direct rules/links, the UI shows the conflict error from `Design::apply`
  (no silent stripping). Dragging a brick onto a normal leaf or composite brick keeps the existing
  composite-nesting gesture. `.brickie/layout.json` keys positions with the `root/` prefix
  (e.g. `root/services`), leaving member brick keys untouched.

## Detailed Implementation Plan

Groups are stored **flat** on `Config` (`Config.groups: Vec<Group>` and `Brick.group: Option<String>`)
instead of synthesising a fake composite `sub: Config`. A grouped brick keeps its canonical path
(`auth`, or `core/auth` inside composite `core`), so grouping or ungrouping does not change its
path, `interface/unused` finding IDs, `brickie check --brick auth`, `brickie interface auth`,
`/docs/auth`, or `.brickie/layout.json` keys. `brickie check --brick services` scopes to the group
and all its member bricks.

### Phase 1: Core model and checker (`crates/core/model`, `crates/core/check`)

1. **Schema & loader** (`crates/core/check/src/config/{schema.rs,load.rs}`):
   - Add `Group { name: String, tags: Vec<String>, desc: Option<String>, loc: String }` and
     `Config.groups: Vec<Group>`.
   - Add `#[serde(default)] pub group: Option<String>` on `Brick`.
   - Concatenate `groups` across `include`d files in `load.rs` alongside `bricks`, filling `loc`.
   - Leave `crates/core/check/src/config/resolve.rs` unchanged (every brick resolves with its own
     declaring directory `base` and `members`; `config/overlap` continues to catch overlaps within
     and across groups).

2. **Unified logical unit** (`crates/core/check/src/config/{schema.rs,selector.rs,decide.rs}`):
   - Add a derived view on `Config`:
     - `pub struct Unit<'a> { pub name: &'a str, pub tags: &'a [String], pub independent: bool, pub is_group: bool }`
     - `Config::unit_by_name(&self, name: &str) -> Option<Unit<'_>>` (returns an ungrouped brick or
       a group; returns `None` for a grouped member brick).
     - `Config::unit_of(&self, brick: &Brick) -> Unit<'_>` (maps a grouped brick to its group, else
       itself).
     - `Config::group(&self, name: &str) -> Option<&Group>` and `Config::group_members(&self, name: &str) -> impl Iterator<Item = &Brick>`.
   - Refactor `Sel::matches`, `Sel::specificity`, `rule_allows`, `judge_link`, `covering_link`, and
     `free` in `decide.rs` / `selector.rs` to operate on `Unit<'_>` instead of `&Brick`.

3. **Validation** (`crates/core/check/src/config/validate.rs`):
   - Enforce all rules in `## Validation` above.
   - Collect known tags from both `cfg.bricks` and `cfg.groups`.
   - In `validate_links` (`child_error`): accept `to = "composite/group"` when `sub.group(child)`
     exists and `sub.exports(child)` is true; reject `group/member` and `composite/grouped_member`.

4. **Rules, exports, cycles, complexity, scoping, todos**:
   - `crates/core/check/src/rules/lift.rs`: add `Config::exports_brick(&self, brick: &Brick) -> bool`
     returning `self.exports(self.unit_of(brick).name)` so `hidden()` treats exported groups as
     visible while still enforcing `exports` on composite members inside a group.
   - `crates/core/check/src/rules/deps.rs`:
     - When lifted ends `from` and `to` share `Some(g)` in `brick.group`, emit `deps/unlinked` with
       rule `"bricks in group '<g>' cannot depend on each other"` and no `link add` help.
     - Otherwise evaluate `covering_link` / `judge_link` using `level.unit_of(from)` and
       `level.unit_of(to)`. For an entered child `c` inside a target composite `sub`, normalise
       `entered` to `sub.unit_of(c).name` so `to = "composite/group"` links match.
     - In `Dep::help`, emit `unit_of(from).name` and `unit_of(to).name`, and omit `/{entered}` when
       the target unit is a group.
   - `crates/core/check/src/rules/private.rs`: in `Private::help`, emit `composite.unit_of(child)`
     so the suggested `export set` command names the group, not a grouped member.
   - `crates/core/check/src/rules/cycles.rs`: skip intra-group edges; run SCCs over
     `unit_of(from).name -> unit_of(to).name`. In `Cycle::owners()`
     (`crates/core/check/src/diag/model.rs`), expand any group name in the cycle to all its member
     brick paths plus the group path so `brickie check --brick <member>` includes the cycle.
   - `crates/core/check/src/rules/complexity.rs`: match selectors at each level against
     `level.unit_of(brick)`.
   - `crates/core/check/src/check/scope.rs`: allow `Filter::new` to resolve group paths (matching
     the group path and all its member brick paths).
   - `crates/core/check/src/check/todo.rs`: add `Todo.groups: Vec<Vec<String>>` (printing
     `"planned group <path> has no bricks yet"`) for 0-member groups; empty member bricks continue
     to report via `Todo.bricks`.

### Phase 2: Design editor & CLI (`crates/core/model`, `crates/core/change`, `crates/cli`, docs)

1. **Ops & wire types** (`crates/core/model/src/design/op.rs`):
   - Rename `BrickKind::Group` → `BrickKind::Composite` and `Op::Ungroup` → `Op::Unnest`.
   - Add `Op::AddGroup`, `Op::SetGroup`, `Op::RenameGroup`, `Op::RemoveGroup`, and optional
     `group: Option<String>` on `BrickSpec` / `Op::SetBrick` (empty string ungroups).
   - Keep `Op::file()` for the primary file and check `Version.files` in `conflict.rs` so
     multi-file `RenameGroup` reports every modified file (the level file, any `include`d files with
     matching bricks/refs, and the parent file for `composite/group` links) to history and conflict
     checks.

2. **Edit engine & diffing** (`crates/core/change/src/`):
   - Implement `edit/group.rs` for `AddGroup`, `SetGroup`, `RenameGroup`, `RemoveGroup`, and update
     `edit/brick.rs`, `edit/moves.rs`, `edit/unnest.rs` (`unnest`), and `edit/via.rs` per
     `### Operation lifecycle`.
   - Reject `--in <group>` in `design/resolve.rs`.
   - Update `edit/element.rs` so semantic diffs include `[[group]]` additions, edits, and removals.
   - Update `report/view.rs` (`design show` text/JSON) and `design/links.rs` (`link_options`,
     `link_check`) to expose groups and exclude grouped member bricks.

3. **CLI & docs** (`crates/cli/src/cli/design/`, `docs/`):
   - Wire `group add|set|rename|rm`, `brick add --kind composite`, `brick add --group`,
     `brick set --group`, `brick nest`, and `brick unnest`.
   - Update [../llms.txt](../llms.txt) (Rule 10, Rules 4–9, schema reference, `brickie design`
     command table), [editor.md](editor.md), [../webui.md](../webui.md), and switch the schema
     snippet in `docs/design/groups.md` from `text` to `toml` so `crates/cli/tests/docs.rs`
     validates it against the loader.

### Phase 3: Layout engine (`crates/lib/algo/layout`)

1. **Variable box sizes** (`crates/lib/algo/layout/src/{lib.rs,rank.rs,compute.rs}`):
   - Add optional `min_w: Option<f64>` and `min_h: Option<f64>` on `InBrick`.
   - In `rank.rs`, compute node width as `base_w.max(min_w)` (not clamped below `min_w`).
   - In `compute.rs`, compute per-row `row_h = max(node.h)` instead of hardcoded `H`, advance row
     `y` by `row_h + gap`, set dummy node height `dh = row_h`, and set `main_bottom` from each
     row's `row_h`.

2. **Channel packing with variable heights** (`crates/lib/algo/layout/src/{channels.rs,ports.rs,smart.rs}`):
   - Key inter-row channels in `channels.rs` by row-band boundaries
     `(rows[r].y + rows[r].h, rows[r + 1].y)` instead of individual port `y` coordinates.
   - Route a vertical stub from any shorter brick's bottom port (`node.y + node.h`) down to
     `rows[r].y + rows[r].h` before entering the horizontal channel so tracks never slice through a
     taller sibling group box in the same row.
   - Update `spans_row` to test strict intermediate rows (`r_from + 1 .. r_to`) rather than raw `y`
     overlap against taller same-row neighbours.
   - Exclude wide/group nodes (`w > MAX_WIDEN`) from `align_single_run` so a wide group does not
     stretch an adjacent single-brick row.
   - Add unit tests in `crates/lib/algo/layout` asserting zero box-edge intersections when a row
     mixes `h = 54` bricks and `h = 180` group boxes.

### Phase 4: Web API, server, and UI (`crates/web/{api,server,ui}`)

1. **API model** (`crates/web/api/src/model/{graph.rs,levels.rs,score.rs,diag.rs}`):
   - Add `GroupMemberView { name, path, is_composite, score, worst_ratio, errors }` and
     `InternalEdgeView { from_member, to_member, kind, count, test, diags }` to `BrickView`, plus
     `is_group: bool`.
   - Update `diag_bricks` (`levels.rs`), `brick_scores` (`score.rs`), and `diag_visible` (`diag.rs`)
     so findings attributed to any member brick path land on the group node without breaking
     single-member selection, and `internal_edges` are counted once per violation (not doubled as
     both `viol_out` and `viol_in`).

2. **Server graph & conflict rules** (`crates/web/server/src/data/`):
   - `graph/mod.rs` & `design/mod.rs`: do **not** emit a `LevelView` for groups.
   - `graph/level.rs`: emit one `BrickView` per logical unit (ungrouped bricks + groups), compute
     `min_w`/`min_h` from member count, populate `group_members` and `internal_edges` (lifting
     intra-group violating edges here instead of dropping them as self-edges), and match
     `projected_links` against `anc_cfg.unit_of(me)` when zoomed into a grouped composite as well
     as `project_link` targeting exported groups.
   - `graph/edges.rs`: lift `End::Outside` targets to the enclosing level's unit so measured ghosts
     and projected link ghosts share one node.
   - `graph/heat.rs`: aggregate member function heat into the group node while retaining per-member
     heat for `GroupMemberView`.
   - `design/conflict.rs`: add `Ident::Group(name)` clashing with `Ident::Brick(name)` on equal
     names, make `SetBrick.group` clash with `RenameGroup`/`RemoveGroup` on the target group, and
     check all files touched by multi-file `RenameGroup`.

3. **Web UI** (`crates/web/ui/src/`):
   - Render group nodes as labelled container boxes showing member cards (with heat/score badges
     and composite zoom affordance), header violation badge, internal red connectors for
     `internal_edges`, and an empty-group placeholder drop zone.
   - Support selecting either the group container or an individual member card (updating URL params
     and inspector scope accordingly).
   - Wire Design-mode drag-and-drop into/out of group boxes (`SetBrick.group`) and group
     creation/editing controls in the inspector.

### Verification

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo clippy -p brickie-ui --features hydrate --target wasm32-unknown-unknown`
- `cargo test --workspace` (including `crates/cli/tests/docs.rs`, layout intersection tests, check
  tests for `Unit`, `composite/group`, intra-group errors, cycles, and change/web conflict tests)
- `cargo run -p brickie -- check --measure --format short` (exit 0)
