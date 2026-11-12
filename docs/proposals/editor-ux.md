# Proposal: design editor UX

Status: **implemented** (2026-11-12) in `crates/web/ui/src/components/editor/` and
`state/editor/`; what differs from this spec is listed in `docs/design.md`, "Design editor",
"Deviations from the spec". This replaces the UI half of `editor.md` ("Editor
(view)", "Editor notes"); the change model, ops, endpoints and agent loop stay as they are. Goal:
an editor a person can use by hand for every field of the config schema, next to the agent,
with direct manipulation on the canvas.

## Problems with the current editor (from screenshots)

1. **The canvas is not the editor.** Linking happens by picking a "kind · scope" chip in the
   right panel and dragging it onto a brick; the canvas offers no handles, no rubber-band edge,
   no visible drop target. Users expect to drag from one brick to another.
2. **Three columns compete; the left one is irrelevant in edit mode.** The findings filters
   (group/level/status/code chips) take the whole left column while designing, and the palette
   and inspector are on the right, far from where the eye is.
3. **Adding a brick shrinks and fades the existing design.** After `add_brick` the level is
   re-laid out tiny and washed out (4-added, 5-inspector); the new brick sits in an
   "(untagged)" lane outside the frame. The full page reload loses zoom and context.
4. **Forms are generic and unconnected.** Inspector uses a separate "Rename" button next to a
   name field plus a "Save" for the rest; tags are a comma string; link chips expose the
   6 kind x scope combinations as flat buttons; the reason prompt is a modal floating over
   unrelated space; there is nowhere to edit rules, complexity, deps defaults, exports or
   includes.
5. **Agent work and change status are opaque.** The ops log is a hidden tab with raw op names
   (`#1 add_brick brick:telemetry`); proposed agent ops are not pinned on the canvas;
   "1 planned" / Preview / Current give no hint of what differs; no measure prompt.

## Principles

- **Canvas first.** Every structural edit (create, connect, move, delete, rename) is a gesture
  on the canvas; panels only refine.
- **One selection, one inspector.** Whatever is selected (brick, edge, rule, level, op) shows
  its full editable schema in the inspector; edits apply on blur/Enter (one op), no Save button.
- **Every edit is visibly an op.** A brief toast "add_rule · brickie.toml · Undo" confirms;
  undo = remove the human op (new `retract` endpoint, see Gaps).
- **Never lose place.** After an op, patch the level in place (keep zoom, pan, selection);
  only new bricks get auto positions.
- **Design vs reality is always legible:** planned = dashed outline, drift = amber, proposed
  agent = violet dashed with a badge, conflict = red striped.

## Layout

Edit mode is a toggle in the header (`E`, button "Design"); entering it opens (or asks to open)
a change. The page keeps its two side columns (user decision): the **left column** stays the
findings/filter panel, the **right column** becomes the design panel. Creation tools do not
take a side column: they live in a **floating toolbar at the top of the canvas**, like
draw.io / Figma / Excalidraw.

```
+--------------------------------------------------------------------------------------+
| brickie  [Architecture] Violations   Design(on)   change: "Add telemetry"  7 ops     |
|          ( Preview | Current | Diff )   2 proposed  1 conflict  1 planned  [Measure] |
+------------------+-------------------------------------------------+-----------------+
| FINDINGS /       |   +-------------------------------------------+ | DESIGN PANEL    |
| FILTERS          |   | ↖  ✋ | ▣ Crate  ▤ Module  ▦ Group | ● uses | | Tabs: Inspector |
| (unchanged)      |   | ◆ impl  ▲ reexp  ✕ deny | prod·test·all |⌫| |  Rules Level    |
|                  |   +------------- floating toolbar ------------+ |  Files Ops      |
|                  |  root > core                      [-] 100% [+] |-----------------|
|                  |  +-------------- core (crates/core/brickie.toml)| INSPECTOR       |
|                  |  |  +--------+   uses     +--------+         | |  name [telem..] |
|                  |  |  | model  |o---------->| facts  |         | |  kind crate     |
|                  |  |  +--------+            +--------+         | |  desc [......]  |
|                  |  |                        +- - - - +         | |  tags [x][+]    |
|                  |  |                        :telemetry: planned | |  members [..]   |
|                  |  |                        +- - - - +         | |  reason ? (todo)|
|                  |  |  exports: model(all) facts(prod)          | |                 |
|                  |  +-------------------------------------------+ |                 |
+------------------+-------------------------------------------------+-----------------+
```

**Floating toolbar** (top of the canvas, centred, draggable/collapsible, stays put while
panning):

| Group | Items | Use |
|---|---|---|
| Tools | Select `V`, Hand `H` | |
| Bricks | Crate, Module, Group | drag a tile onto the canvas (ghost follows), or click it then click/drag a box on the canvas |
| Links | uses, implements, reexports, deny | the link type is chosen here or by the typed port on a brick (see Connecting); clicking one arms the link tool |
| Scope | prod · test · all | link mode applied to new rules (`1/2/3`) |
| Delete | ⌫ | delete selection |

The design panel tabs (right column, all scoped to the **current level**):


| Tab | Content |
|---|---|
| Inspector | the selection (brick, edge, frame); level properties when nothing is selected; tag swatches |
| Rules | `[deps]` default/cycles toggles; dep rules table (from, to, kinds, scope, action, reason, help) with drag to reorder disabled (order irrelevant), each row showing its physical file; complexity rules table |
| Level | this level's file, `exports` editor (child + scope), parent link, "make this a preset" |
| Files | include list of this level's file (add/remove; create preset), shared-file warning |
| Ops | change log grouped by element, filters (agent/human, proposed/conflict) |

## Interactions

### Bricks

| Gesture | Result | Op |
|---|---|---|
| Drag palette tile onto empty canvas | ghost box follows; drop -> inline name field on the box (module: `crate::mod` hint prefilled) | `add_brick {kind}` + `set_layout` |
| Drag palette tile into a **tag lane** | lane glows "→ tag: lib"; the name box says "tag: lib" | `add_brick {tags: [lane]}` (layout places it in the lane) |
| `B` then click canvas / double-click empty canvas | same, kind from last used (Tab cycles kind) | `add_brick` |
| Drop palette tile **onto a brick** | brick highlights as a folder "into web (3 inside)" / "(becomes a group)"; name box says "into web"; you stay on this level | `make_composite` (if leaf) + `add_brick` in child file (one undo step) |
| Double-click name / `F2` | inline rename on the box | `rename_brick` |
| Edit desc/tags/members/path in inspector, blur | — | `set_brick` (only changed fields) |
| Drag a tag swatch onto a brick / click `x` on a tag pill | tag added / removed | `set_brick {tags}` |
| `Del` / toolbar ⌫ / context "Delete" | confirm popover listing rules and exports that reference it | `remove_brick` |
| Inspector "Make group" on a leaf | asks dir (default name) | `make_composite` |
| Inspector "Ungroup" | children lifted to the parent level, brick stays a leaf | `ungroup` |
| Drag brick to empty space / within its own lane | position stored | `set_layout` (not an op) |
| Drag brick into **another tag lane** | lane glows "→ tag: lib" | `set_brick {tags}`: first tag replaced by the lane's, others kept; "(untagged)" lane drops the first tag |

### Moving between levels

| Gesture | Result | Op |
|---|---|---|
| Drag brick onto a sibling | target glows at once as a folder (lid tab), label "into <b>"; drop | `move_brick` via `ChangeMove {from, into}` (a leaf becomes a group) |
| Drag brick onto the level frame edge / breadcrumb parent ("drop here = move to parent") | moves up one level | `move_brick` into parent file |
| Drag brick onto any breadcrumb segment | moves to that ancestor level | `move_brick` |
| Cut `Ctrl+X`, navigate, paste `Ctrl+V` | move across non-adjacent levels (e.g. into a cousin); with a brick selected, into it (implemented: `editor/clip.rs`; the hint shows what is cut, Esc forgets it) | `move_brick` |
| Rest a dragged brick on a group ~700 ms | the group's level opens with the brick still on the pointer (spring-loaded, see below) | `move_brick` on release |
| Drop result | toast: "Moved; code is not moved on publish (manual step)" and N rules rewritten / exports dropped | — |

Invalid targets (into itself or a descendant, onto a faded neighbour) show a red outline, a
not-allowed cursor and the reason; the drop is ignored and the brick goes back.

### Drag and drop: folders and lanes (implemented 2026-11-12)

Mental model: **a brick is a folder, a tag lane is a shelf.** Dropping on a brick puts the
thing inside; dropping in a lane files it under that tag; dropping on empty canvas only
places it.

What lies under the pointer decides, in this order (the first match wins):

| # | Under the pointer | Existing brick dragged | New tile dragged | Highlight / ghost label |
|---|---|---|---|---|
| 1 | a brick body (this level) | `move_brick` into it | `make_composite` (if leaf) + `add_brick` inside | folder fill, lid tab; "into web (3 inside)" / "(becomes a group)" |
| 1 | the dragged brick itself, a faded neighbour | refused | refused (neighbour) | red; "✕ reason"; not-allowed cursor |
| 2 | "+ new tag lane" zone (under the last lane, shown while dragging) | tag asked inline, then `set_brick {tags}` | name and tag asked inline, `add_brick {tags}` | dashed band; "new tag lane…" |
| 3 | frame edge band / a breadcrumb | `move_brick` up | ignored (falls through) | frame dashed; "up to root" |
| 4 | a tag lane (nearest row inside the frame) | other lane: retag; own lane: position only | `add_brick {tags: [lane]}` | lane band; "→ tag: lib" / "remove first tag" |
| 5 | empty canvas | position stored | `add_brick` untagged, at the drop point | "move here" / "place here" |

Decisions (ambiguous points):

- **No hold delay.** Folder targeting is immediate, like Finder / Explorer; the target shows
  what will happen, and Esc or moving away cancels, so a hold adds latency without safety.
- **Retag replaces the first tag only.** The first tag is the lane a brick is drawn in; other
  tags are rule selectors the user set on purpose. A lane already among the other tags is not
  duplicated. Untagged lane = drop the first tag.
- **A lane drop lets the layout place the brick** (its stored position is cleared), so it lands
  tidily in the lane instead of where the pointer was; a drop on empty canvas or in its own
  lane keeps the exact position.
- **A tile dropped in a brick does not navigate.** You stay on the level; the folder's child
  count updates and the toast says "Added sub into cli · Ctrl+Z undoes".
- **One gesture, one undo step.** A tile into a leaf records two ops; Undo retracts both, Redo
  restores both (the page keeps the gesture's seq range).
- **The tile ignores the frame edge** (a new brick cannot go "up"); breadcrumbs too.
- **Spring-loaded zoom** (implemented 2026-11-12, `editor/spring.rs`): resting a drag on a
  group brick for 700 ms opens its level, macOS-style; the folder fill charges while the hold
  runs and the label adds "· hold to open". A brick dragged on its own level is lifted off it
  (it returns to its place there) and its ghost keeps following the pointer on the new level:
  drop into a brick (or rest again to go deeper), on the frame edge / a breadcrumb (up) or
  anywhere else (onto this level, untagged; lanes and the new-lane zone do not apply to a
  lifted brick). A tile carries over the same way. Esc cancels; the opened levels are
  ordinary history entries.
- **Feedback:** target highlights fade in (120 ms; none with `prefers-reduced-motion`); the
  result label rides on the tile ghost or under the dragged brick; after a drop a toast names
  the op(s) and "Ctrl+Z undoes".

### Connecting (dependency rules)

**Type first** (user decision): people think of the kind of link before the target, and a form
shown after the drag is easy to skip. So the link type is chosen *before* dragging, by picking
which connection port to drag from; nothing has to be filled in after the drop.

**Typed ports.** Hovering or selecting a brick shows a port strip on its right edge, one port
per link type, each with its own colour and glyph (legend in the floating toolbar):

```
            +-------------+ ●  uses        (solid arrow)
            |  telemetry  | ◆  implements  (hollow triangle head)
            |             | ▲  reexports   (double head)
            +-------------+ ✕  deny        (red, bar end) - same kinds, action deny
```

- The port fixes **kind** (and **action** for the deny port). **Scope** comes from the link
  mode in the floating toolbar (`prod | test | all`, default `all`, keys `1/2/3`), shown on the ports.
- Drag from a port -> rubber band in that type's style; valid targets (siblings,
  `link_options`) highlight, and a target where this link would change nothing shows
  "already allowed" and is not droppable; dropping there says which rule decides it, by its
  path from the root config's directory ("No rule added for web → lib: already allowed by
  brickie.toml:62").
- **Drop = rule created** immediately (`add_rule`), no blocking form. The new edge is selected
  and the inspector focuses its `reason` field; typing is optional.
- A rule without a reason gets a small "?" marker on the edge and a "rules without reason"
  item in the change bar and `brickie change --todo`, so a forgotten reason is visible later
  rather than blocking now. Publish warns (does not block) when reasons are missing.

| Gesture | Result | Op |
|---|---|---|
| Drag port (type T) A -> B | rule A->B of type T, current scope | `add_rule` |
| Drag port onto a **tag lane** / tag swatch | `to = "tag:<t>"` | `add_rule` |
| Drag a port shown on a tag lane header | `from = "tag:<t>"` | `add_rule` |
| `L` (link tool) | ports shown on every brick, not only on hover | — |
| Click an edge | selects; inspector: kinds, scope, action, reason, help, deciding rules (with file), measured uses | — |
| Edit fields in inspector / Rules tab | — | `set_rule` |
| Change an edge's type | inspector kind chips, or drag a different port onto the same target (merges kinds) | `set_rule {kinds}` |
| `Del` on a rule-edge | removes that rule (not measured code edges) | `remove_rule` |
| Drag an edge's arrowhead to another brick | retarget (implemented: the selected rule shows a handle on each arrowhead, above the port strips; only `to` changes) | `set_rule {to}` |

Several kinds on one rule are shown as one edge with stacked glyphs; editing kinds in the
inspector splits nothing. The inspector's "Now" line comes from `link_options` (deciding rule
or `deps.default`).

Edge rendering in edit mode: measured edge solid grey; rule-allowed planned edge dashed green;
deny rule red with a bar end; violation red solid; agent-proposed violet dashed with an `A`
badge. Rules whose selectors are tags draw as a fan from the tag lane header.

### Levels and zoom

| Gesture | Result |
|---|---|
| Double-click group / `Enter` on selected group | zoom into its level (breadcrumb updates) |
| `Backspace` / click breadcrumb | out to parent |
| Ctrl+wheel / pinch / `+ -` / `0` fit | zoom canvas; at high zoom children of groups render as preview (map-style, as in view) and accept drops |
| Drop on a nested child shown in map-style zoom | add or move directly into that deeper level |

### Exports (composite boundary)

The level frame shows an "exports" strip along its bottom edge. Drag a child onto the strip to
export it (scope `all`); click the pill to cycle `all -> prod -> test`; drag off or `x` to
remove. Each change records one `set_exports` with the full list. Children not exported get a
small lock icon.

### Deps defaults, complexity, presets

- Rules tab header: `default ( allow | deny )`, `cycles ( allow | deny )` -> `set_deps`.
  Switching default to deny shows a preview count: "would add 14 deps/deny findings".
- Complexity table: one row per rule: `bricks` selector (brick/tag picker), cognitive,
  cyclomatic, nesting, paths, params, lines (empty = unset), level (error/warn), scope, help.
  `+` -> `add_complexity`; edit cell -> `set_complexity`; row `x` -> `remove_complexity`.
  Selecting a brick also lists "complexity rules that apply" with a link to the row.
- Files tab: includes of the current file. `+ include` picks an existing toml or "New preset..."
  (asks path) -> `create_file` + `add_include`; `x` -> `remove_include`. Clicking a preset opens
  its rules in the Rules tab with a banner "editing <file>, included by N levels".
- Every rule row shows its physical file; rows from an included file are tinted, and editing
  them warns "shared by N levels".

## Agent ops

- **On canvas**: every element touched by a proposed agent op carries a violet `A` badge and a
  dashed violet outline (bricks) or violet dashed line (rules). Removed elements stay drawn
  ghosted with strike-through until reviewed. Hover shows the op note.
- **Review inline**: selecting such an element puts a review bar at the top of the inspector:
  `Agent proposed: add_rule telemetry -> lib (uses, prod) "note"   [Accept] [Reject...]`.
  Reject asks a note (required). `A`/`R` keys accept/reject the selected proposal; `N` jumps to
  the next proposal (pans and zooms across levels).
- **Ops tab**: grouped by element in plain words ("Added brick telemetry", "Allowed telemetry
  -> lib (uses, prod)"), author chip, state chip, file. Click = select on canvas. Header has
  "Accept all from agent" (with confirm).
- **Conflict**: red striped outline + `!` badge on canvas; the op row explains ("rule #3 changed
  since; expected X, found Y") with actions Reject, or Re-apply-as-mine (Gap: needs a
  rebase helper). Publish is blocked and the button says why.
- Changes made elsewhere reach the page through Server-Sent Events (`/api/events`, see
  `design.md` Serve "Events"): an agent's op is patched in place within a second; a re-measure
  or a config edit outside the change shows a "… · Refresh" pill instead of reloading.
- New agent ops arriving show a toast "Agent proposed 2 ops [Show]"; unseen ones pulse once.

## Change bar, preview, publish

- **Preview | Current | Diff**: Diff overlays both: added green, removed red ghost, changed
  amber; hovering an element shows before/after fragments.
- Chips are buttons: `2 proposed` -> Ops filtered, `1 planned` -> select planned bricks,
  `drift` -> findings in the left column.
- **Measure prompt**: when the cache is older than the last code edit, or after publish
  scaffolding, a bar "Design checked against code measured 12 min ago [Measure now]"
  (`/api/measure`); the button shows progress and refreshes planned/drift.
- **Publish...**: dialog with three sections: files written (diff per file, collapsible),
  files scaffolded, manual steps (moves); blockers at top with links to the elements. Primary
  button disabled until no blockers.
- **Discard**: confirm with op count; type-to-confirm when agent ops are accepted.

## States and feedback

| State | Visual |
|---|---|
| Hover brick | lift shadow, connection handles, name tooltip with file |
| Selected | 2px accent outline, inspector populated |
| Dragging brick | 60% ghost follows; original stays dimmed in place |
| Valid drop target (into a brick) | folder: accent fill, lid tab, label "into X (N inside)" |
| Lane drop target | lane band accent, label "→ tag: T"; "+ new tag lane" zone under the last lane |
| Parent drop zone | frame edge highlights, label "Move to <parent>" |
| Invalid target | red outline, "no" cursor, reason tooltip |
| Linking | rubber-band arrow; valid siblings outlined green; existing rule shows "already allowed by #n" |
| Planned brick | dashed outline, "planned" tag |
| Drift (code not in any brick) | amber chip on level frame "3 modules unassigned" -> list, drag one onto a brick = `set_brick {members}` |
| Proposed (agent) | violet dashed + `A` badge |
| Conflict | red stripes + `!` |
| Op in flight | element spinner; on 409 revert and show engine message inline |

## Empty and error states

- **No change open**: Design toggle opens a small dialog "What is this change for?" (intent) ->
  `ChangeOpen`. Read-only users (non-loopback) see the toggle disabled with the reason.
- **Empty level** (new group): centered hint "Drag a Crate, Module or Group here" with the
  palette tiles inline.
- **No rules**: Rules tab shows defaults and "Drag between bricks on the canvas to add a rule".
- **Preview fails to load**: canvas shows the current design greyed, a red banner with the
  loader error and the op that caused it (link) and "Retract" on it.
- **409 on an op**: inline error at the gesture site, nothing changes.
- **Op recorded but the design no longer checks** (e.g. a moved brick's members fall outside
  its new composite): the reply has no levels and `change.error` says why; the error line
  shows it with "Ctrl+Z undoes" (the canvas keeps the last good preview).
- **No change open**: the intent field is focused; Enter opens, Esc cancels.
- **Stale measure / no measure**: measure prompt (above).

## Keyboard

| Key | Action |
|---|---|
| `E` | toggle edit mode |
| `V` `B` `L` `M` | select, brick, link, move tools |
| `Tab` (brick tool) | cycle crate/module/group |
| `F2` / `Enter` | rename / zoom into group |
| `Backspace` | out to parent level |
| `Del` | delete selection (with confirm popover) |
| `Ctrl+X` / `Ctrl+V` | move brick across levels |
| `Ctrl+Z` | undo: retract the last human op (a multi-op gesture whole); also the change bar's Undo |
| `Ctrl+Shift+Z` / `Ctrl+Y` | redo: restore the last retracted op (gesture whole); also Redo, whose tooltip names it |
| `Alt+←` / `Alt+→`, mouse 4 / 5 | Back / Forward through the view history (level, selection) = browser history; `←` `→` buttons by the breadcrumb, tooltip "Back to root/core"; not while typing |
| `A` / `R` / `N` | accept / reject / next proposal |
| `P` `C` `D` | Preview / Current / Diff |
| `Esc` | cancel drag (the brick goes back), popover, tool, cut brick, shortcuts sheet |
| `?` | shortcuts sheet: every key and gesture (also the toolbar's `?` button) |
| `/` | find |
| `+ - 0` | zoom, fit |

## Coverage matrix

| Schema field / op | Affordance |
|---|---|
| `brick.name` | inline name on create; F2 rename -> `add_brick` / `rename_brick` |
| `brick.kind` (op only) | palette tile |
| `brick.members` | inspector chip list with selector-type picker (package, `crate::mod` prefix, `module:` exact, `path:`), autocomplete from measured modules; drag unassigned module onto brick -> `set_brick` |
| `brick.tags` | tag pills + swatch drag -> `set_brick` |
| `brick.path` | "Make group" (`make_composite`); path field for composites -> `set_brick {path}` |
| `brick.desc` | inspector textarea -> `set_brick` |
| remove brick | Del -> `remove_brick` |
| move brick | drag into / to parent / cut-paste -> `move_brick` |
| `exports` (name, scope) | frame exports strip -> `set_exports` |
| `deps.default`, `deps.cycles` | Rules tab toggles -> `set_deps` |
| `deps.rule.from/to` (brick) | canvas drag between bricks, arrowhead retarget |
| `deps.rule.from/to` (`tag:`) | drag from/to tag lane or swatch; selector picker in Rules table |
| `deps.rule.kinds` | typed port dragged from; inspector kind chips (none = all) |
| `deps.rule.scope` | scope in the floating toolbar (prod/test/all) before dragging; inspector |
| `deps.rule.action` | deny port vs kind ports; inspector toggle |
| `deps.rule.reason`, `help` | inspector (focused after drop); "?" marker and todo when missing |
| add/set/remove rule | `add_rule` / `set_rule` / `remove_rule` |
| `complexity.*` (bricks, 6 limits, level, scope, help) | Rules tab complexity table -> `add/set/remove_complexity` |
| `include` | Files tab -> `add_include` / `remove_include` |
| new preset | Files tab "New preset" -> `create_file` + `add_include` |
| review agent op | canvas badge + inspector review bar + Ops tab -> `ChangeReview` |
| publish / discard | change bar -> `ChangePublish` / `ChangeDiscard` |
| layout | drag -> `ChangeLayout` |

## Gaps (schema or engine work needed)

Status (2026-11-12): 1, 2, 5, 6, 7 and "rules without reason" are implemented in the engine and
the server API (see `editor.md` "Editor UX gaps" and "Endpoints") and used by the UI. The UI
also needed the rules, complexity rules, exports and includes of each file:
`ChangeView.sheets` (`FileSheet`).

1. **Undo/retract** a human op: done: `ChangeRetract {seq?}` (`None`: the latest human op) adds
   a review with verdict `retract`; the op counts as rejected, the log is not rewritten.
2. **Ungroup** (composite -> leaf, lifting children up): done: `ungroup {file, name}` op.
   Decision: no `remove_file` op; publish never deletes, it lists the dropped file as a manual
   step (a file created in the change just disappears).
3. **Reorder bricks / rules**: not needed by semantics; not offered.
4. **Rule from/to selector validation** (typos in `tag:`) should be checked client-side from the
   level's tags; engine already rejects unknown on load.
5. **Planned edges**: done: `LevelView.planned_edges` and `ChangeView.planned_edges`
   (`PlannedEdge`: level, file, rule index, from/to selectors, kinds, scope, action, reason,
   help, seq/by/state of the op), and `link` tasks of `--todo`.
6. **In-place patching**: done on the server: op, move, review, retract and re-apply answer
   `ChangeReply {done, change, levels}`; `ChangeLevels {paths}` fetches levels. The
   "already allowed" refusal uses `ChangeLinkCheck`; selector validation uses
   `ChangeView.levels[<level>].selectors` (gap 4).
7. **Conflict re-apply**: done: `ChangeReapply {seq}` re-records the op as a human op against
   the current design and rejects the original.
8. Removing a measured code edge is not a design op (code change); the UI says so on Del.
- **Rules without reason**: done: `reason` tasks (`change/no-reason`) in `Change::todo` /
  `--todo` and `Plan.warnings` (not a blocker).

## Implementation order

1. **Layout** (`components/editor/mod.rs`, `bar.rs`, new `editor/toolbar.rs`, `viewer.css`):
   floating toolbar over the canvas (tools, brick tiles, link types, scope, delete); right
   column = design panel (Inspector, Rules, Level, Files, Ops); left column unchanged.
   Change-bar chips become buttons; Diff toggle.
2. **In-place refresh** (`state/editor.rs`): fetch level preview after an op instead of reload;
   keep zoom/selection; toast with op name.
3. **Canvas create/rename/delete** (`palette.rs`, `drop.rs`, `canvas/brick.rs`, `overlay.rs`):
   draggable tiles with ghost, inline name editor on the box, F2, Del popover.
4. **Canvas connect** (`canvas/pointer.rs`, `canvas/edge.rs`, new `editor/connect.rs`):
   typed ports per link kind, scope in the floating toolbar, rubber band, instant add_rule (replaces link chips and reason dialog in
   `inspector.rs`/`dialogs.rs`); edge selection, retarget, delete; tag lane targets.
5. **Move between levels** (`drop.rs`, `canvas/frame.rs`, breadcrumb): hold-to-move-into,
   frame/breadcrumb drop zones, invalid targets, cut/paste.
6. **Inspector rewrite** (`inspector.rs`): auto-apply fields, members chip editor with
   autocomplete, tag pills, make group, exports strip on `canvas/boundary.rs`.
7. **Rules / Level / Files tabs** (new `editor/rules.rs`, `editor/complexity.rs`,
   `editor/files.rs`): deps toggles, rule and complexity tables, includes and presets.
8. **Agent review on canvas** (`log.rs`, `canvas/brick.rs`, `canvas/edge.rs`): badges, inspector
   review bar, plain-language op rows, next-proposal navigation, conflict display.
9. **Measure prompt and publish dialog** (`controls/measure.rs`, `dialogs.rs`).
10. Engine gaps 1, 5, 7 alongside phases 2, 4, 8.

Each step keeps files under ~300 lines and needs the UI clippy check for `hydrate`.
