# Proposal: design editor and change sessions

Status: **agreed**, reconciled from two independent reviews (config model; agent UX and viewer
feasibility) and the user's decisions of 2026-11-12. Phases 1, 2, 3, 4 and 6 **implemented**
(see "Implementation notes" and "Editor notes"); phases 5 (rules panel, presets, includes) and 7
(MCP) open.

## Idea

A draw.io-like editor in the viewer: add bricks, zoom into a brick and add children, connect
bricks, write descriptions, edit rules. Design work happens in a **change**: a pending, ordered
log of operations over the `brickie.toml` tree that one human and one coding agent work on
together. The agent learns what the human is doing (and the reverse) through `brickie change`,
which shows what changed, by whom, how it was reviewed and what is not built yet.

User decisions (fixed):

1. The `brickie.toml` tree stays the single source of truth for the agreed design.
2. **Nobody writes `brickie.toml` directly** while a change is open: the editor (human) and the
   CLI (agent) both record operations; the toml files change only on publish.
3. Change tracking is brickie's own and independent of git.
4. One change open at a time, one human and one agent.
5. Code edits are out of scope: the change covers the design only; the agent edits code as usual.

## Model

### Storage (`.brickie/change/`, git-ignored with the cache)

- `base/`: the raw bytes of every file of the config tree when the change was opened (each
  `brickie.toml`, every included file, `brickie.review.toml`), keyed by workspace-relative path.
- `ops.json`: append-only list of ops.
- `intent`: free text, what this change is for.
- `readers.json`: last `seq` read per reader (`human`, `agent`).
- `lock`: writers (server and CLI) take the lock, assign `seq` under it and write by atomic
  rename.

### Op record

`{seq, by: human|agent, at, op, file, key, before, after, note, state, review}`:

- `file`: the **physical** file the op edits (never "the level": included files are merged into
  every level that includes them, `config/load.rs`). The editor warns when a file is shared.
- `key`: brick name, or a rule's index within `file`; `before` is the canonical TOML fragment it
  must match, `after` the new fragment (lossless in JSON output).
- `state`: `proposed` | `accepted` | `rejected`. Human ops are accepted when recorded; agent ops
  stay proposed until reviewed. `review = {seq, by, note}`: reviewing appends a new `seq`, so a
  reader following `seq` also sees state changes and the reason for a rejection.
- Ops are never rewritten; repeated edits of one field are coalesced only on display.

### Operations (cover `config/schema.rs`)

- Bricks: `add_brick`, `remove_brick`, `set_brick` (desc, tags, members, path),
  `rename_brick` (also rewrites rule selectors, parent/child exports and accepted review ids;
  a remove + add would lose them).
- Levels: `make_composite` (sets `path`, creates `<dir>/brickie.toml`, optional exports; one op
  because a `path` without its file is `config/missing`), `set_exports` (the top-level, scoped
  `exports` of a composite's own file).
- Rules: `add_rule`, `remove_rule`, `set_rule` (from, to, kinds, scope, action, reason, help),
  `add_complexity`, `remove_complexity`, `set_complexity`, `set_deps {default, cycles}`.
- Files: `add_include`, `remove_include`, `create_file`. A preset is a rules file other configs
  include; creating one is part of the change (`create_file` + rule ops on it + `add_include`).

### One engine for preview and publish

- `load` gets a small file-source trait (today it uses `std::fs` and `canonicalize`).
- **Preview** = the `base/` files as `toml_edit` documents in memory, the ops replayed in `seq`
  order (accepted, optionally + proposed), then the normal loader and `check` over that virtual
  file set. Each op's `before` is checked against the preview at its own `seq`; a mismatch marks
  it `conflict`. Rejecting an op re-checks later ops; those that no longer replay become
  `conflict` and are shown to both sides.
- **Publish** writes those same documents to disk (comments and order kept, as `review/edit.rs`
  already does) and then scaffolds (below). It refuses when: an op is still proposed or in conflict; a touched file's bytes
  differ from `base/` (edited outside brickie); or the result fails `load`/validation. Then the
  change is closed. `discard` deletes it.
- **Scaffolding on publish** (user decision): for each brick added or made composite whose code
  does not exist yet, publish creates it: the folder, a crate (`Cargo.toml`, `src/lib.rs`) or a
  module file (`mod.rs` plus the `mod` line in the parent) as the brick's members say, and the
  crate in the workspace `members`. The publish dialog lists every file it will create or touch
  before writing; existing files are never overwritten. A scaffolded brick is still *planned*
  until the next `measure`.

### Planned, built, drift (preview, from `check --change` diagnostics)

- **planned**: a brick whose members/selectors match no measured module (a composite whose
  `path:` crate does not exist yet); an allowed edge the design adds but no measured module edge
  uses.
- **drift**: measured modules no brick covers or no child takes (`config/unassigned`-style), and
  `deps/deny`, `deps/private`, `deps/cycle` findings on preview edges.
- The change is **built** when nothing is planned, there is no drift and `check --change` has no
  errors. All of this depends on a fresh `measure`: outputs show the cache age and `stale/*`.
- New codes (`change/planned`, `change/conflict`, ...) go into `codes.rs` and `docs/llms.txt`.

## Agent interface

New core crate `crates/core/change` (ops, log, overlay, publish), used by the CLI, `check` and the
server; the op enum is shared with `crates/web/api`.

| Command | Output |
|---|---|
| `brickie change` | intent, status, cache age, semantic diff vs base by element with author and notes, and **what is new since this reader's last read** (then advances its cursor) |
| `brickie change --since <seq>` | explicit cursor, does not advance |
| `brickie change --todo` | concrete tasks from `check --change`: create the module for planned brick X at `<path>`, edge X→Y planned, drift Z (finding id, spans, help), op N rejected: `<note>` |
| `brickie change propose --json` | an op object on stdin (same schema as the endpoint); records a proposed op |
| `--format text\|json` | on all; JSON schema documented in `docs/llms.txt` |

Humans can attach notes to bricks and edges (the `note` of an op, the `reason` of a rule); they
show inline in the diff and are the agent's instructions. The agent polls (`brickie change` before
each step); an MCP server with notifications comes later.

`docs/llms.txt` gets a "Change sessions" section: when a change is open, never edit toml, only
propose; read `brickie change` at the start and before each step; after code changes run
`measure` + `check --change`; stop and ask when an op is rejected or in conflict. Its current
"Hard constraints" are reworded to match.

## Editor (view)

- A mode of the existing canvas (`Params.mode=edit`) reusing levels, zoom and layout.
- After an op the client fetches the preview for the current level (`Graph` with `change=1`) and
  relays out that level only; no page reload. `layout::Input` gets explicit planned bricks and
  planned edges, drawn dashed.
- Canvas actions map to ops: add brick, add child after zooming in (`make_composite` when
  needed), drag to connect (asks for a reason), edit desc/tags/exports, rename, delete. A Rules
  panel edits deps, rules, complexity rules and includes, showing the physical file of each.
- Agent ops are styled apart with accept/reject (with a note); the change bar holds intent,
  publish, discard. The UI polls ops since its last `seq`.
- Write endpoints (`ChangeOp`, `ChangeReview`, `ChangePublish`, `ChangeDiscard` in
  `crates/web/api`) and the existing `POST /review` share one guard: loopback bind and an
  `Origin`/`Host` check against CSRF.

## Decided questions

1. Scaffolding: done by the editor on publish (see above).
2. "Edit preset" means an included rules file, not the view preset `p=`; a change can create and
   edit presets.

## Phases

1. `crates/core/change`, the file-source trait in `load`, ops, preview/publish, `check --change`,
   the `brickie change` CLI (human review via CLI too). Proves the agent loop without canvas work.
2. `llms.txt` section and codes.
3. Editor mode: edit existing bricks and rules, review agent ops, publish/discard.
4. Add bricks/edges, composites and renames on the canvas; planned layout.
5. Rules panel, presets and includes.
6. Scaffolding on publish.
7. MCP.

## Implementation notes (phases 1, 2, 6)

- Types: `Op`, `OpRecord`, `Entry` (op or review, one `seq` each), `BrickKind`, `Status`,
  `DiffEntry`, `Task`, `Plan`, `LinkOption`, `LayoutHint` are plain serde data in
  `brickie_model::change` (wasm-safe; `crates/web/api` can use them). The engine is
  `crates/core/change` (`Change::{open, load, record, review, log, since, preview, status,
  diff, todo, publish_plan, publish, discard, move_op, link_options, link_check, retract,
  reapply, planned_edges, reasonless_rules, layout, set_layout}`).
- `ops.json` holds both op records and review entries; an op's `state` / `review` are derived
  from its last review, so no entry is rewritten. `conflict` is derived on every read by
  replaying accepted and proposed ops. Only a human reviews.
- `key` is `brick:<name>`, `rule:<i>`, `complexity:<i>`, `exports`, `deps`, `include` or `file`;
  an added element has `before: null`, which always replays (a duplicate brick name becomes a
  conflict). An op that does not apply when recorded is an error and is not logged.
- Ops address physical files. The editor works with brick paths: `Change::move_op(from,
  to_parent)` turns paths into a file-based `move_brick`.
- User steering (2026-11-12): the editor draws bricks by **kind** (`crate`, `module`, `group`):
  the kind stays in the op and only fills in `members` / `path` (a group creates its file).
  Links are drawn from `Change::link_options(path)` (kinds x prod/test per sibling, with the
  deciding rule) and recorded as `add_rule`. A brick can be **moved into** another
  (`move_brick`; the target is made a composite when needed, `path:` members are rebased, the
  source file's rules and exports on it are rewritten to the new parent when it is a sibling,
  otherwise its exports entry is dropped). **Publish moves no code**: a moved brick is a `skip`
  item of the plan (a manual step). Canvas positions are not ops: `layout.json` in the change
  keeps them (`layout` / `set_layout`).
- Deviations: rename and move do not rewrite accepted review ids (an id is a hash of the finding,
  which changes with the brick name; the old acceptance simply stops matching). Planned edges
  are the rules the change *adds* (canonical text not among the base rules of the same file)
  that no measured module edge backs at their level (`Change::planned_edges`, `PlannedEdge`
  with level, file, rule index, selectors, kinds, scope, action, reason, help and the last op
  that wrote it); the allow ones are `change/planned-edge` `link` tasks of `--todo`. Rules
  the change adds without a `reason` are `change/no-reason` `reason` tasks and publish
  `warnings` (not blockers). `check --change` checks accepted plus proposed ops.
  Scaffolding matches new bricks by name; a package member that is not measured becomes a crate
  at `<dir of the file>/<brick name>/`.

- Editor UX gaps (2026-11-12, `editor-ux.md` "Gaps"): **retract** is a review entry with
  verdict `retract` on a human op (the op counts as rejected; the log stays append-only;
  `Change::retract(None)` takes the latest accepted human op). **Re-apply** re-records a
  conflicting op's `Op` as a human op against the current design and rejects the original
  (`re-applied as op N`); it fails, recording nothing, when the op does not apply now.
  **Ungroup** is an op: children, rules and complexity rules of the composite's file move up
  (`path:` members rebased), its exports are dropped, the brick stays a leaf without `path`;
  refused when the child file has `include` or `[deps] default`/`cycles`. Decision on the
  file: publish never deletes; the dropped file is a `skip` item ("delete it by hand"), a
  file created in the change just disappears. **Link check** (`Change::link_check(level,
  Link)`) says a rule is redundant when an equal rule exists or every edge it covers (matching
  pairs x kinds x scopes) is already decided as asked.

## Editor notes (phases 3, 4: the web half)

- **Endpoints** (`crates/web/api/src/endpoint/editor.rs`, handlers in
  `crates/web/server/src/routes/editor.rs`): `ChangeState` GET `/api/change` (status, log,
  diff, todo, layout, level files, preview error; `null` when none is open), `ChangeOpen`,
  `ChangeOp` (a human op, accepted when recorded), `ChangeMove` (`{from, into}` brick paths ->
  `Change::move_op` -> recorded), `ChangeReview` (accept/reject with a note), `ChangeRetract`
  (`{seq?}`), `ChangeReapply` (`{seq}`), `ChangeLinks` and `ChangeLinkCheck` (read only),
  `ChangeLevels` (`{paths}` -> marked preview levels, read only), `ChangeLayout`,
  `ChangePublishPlan` GET, `ChangePublish`, `ChangeDiscard`. Op, move, review, retract and
  re-apply answer `ChangeReply {done, change, levels}`: the change view after the write and
  the preview levels of the files it edited, their ancestors and direct children, for
  in-place patching. The change view also carries `levels` (per level: bricks, tags and valid
  selectors) and `planned_edges`; each preview `LevelView` carries its `planned_edges`. A
  refused change operation is a 409 with the engine's message.
- **Preview in the page.** `change=1` (a `Params` field) makes the page, `/api/graph`,
  `/api/diagnostics` and `/api/functions` load the change's preview (`Preview::check`) and run
  the same level aggregation over it; bricks carry `planned` (no measured module,
  `brickie_change::planned`) and `proposed` (added or renamed by an agent op still proposed).
  Planned bricks have no edges; the layout takes them as edge-less boxes. A preview that does
  not load falls back to the current design and the change bar shows the error.
  `PageData.change` embeds the change view, so the editor needs no extra request on load.
- **Write guard** (`routes/guard.rs`, an axum extractor on `/review` and every writing change
  endpoint): the server must be bound to loopback, the `Host` must name a loopback address
  with the server's port (DNS rebinding), and an `Origin`, when sent, must equal
  `http://<Host>`; without `Origin`, `Sec-Fetch-Site` must not be cross-site. Otherwise 403.
- **UI** (`crates/web/ui/src/components/editor/`, state in `state/editor.rs`): the header's
  Design button opens a change (intent dialog); while one is open the change bar replaces the
  banner (intent, op/proposed/conflict/planned chips, Preview/Current, Publish with the plan
  dialog listing every file and blocker, Discard). In Preview the right panel is the editor:
  the palette (Crate / Module / Group: pick, then click or drop on the stage; the name, and
  for a module its `crate::module` member, is asked inline; the position is stored with
  `set_layout`), the Inspector (rename, description, tags, delete, "Open its level" for a
  group, link chips kind x scope with an allow/deny choice and the table of what the rules
  decide now; drag a chip onto a sibling, or pick it and click the sibling, then give a
  reason -> `add_rule`) and the Ops log (newest first, agent ops styled apart, proposed ones
  with Accept/Reject and a note). Dragging a brick onto another asks to move it inside
  (`ChangeMove`); dropped elsewhere its position is stored. Planned bricks are dashed, agent
  proposals dashed in the agent colour. Escape drops an armed tool, link or open form.
- **Deviations.** After each op the page reloads (the server's preview is the only truth; the
  URL keeps level and selection) instead of patching the canvas in place; the server now
  answers the touched levels (`ChangeReply`) and the planned edges, the UI does not use them
  yet (`editor-ux.md`). Ops are recorded as the human only; the agent uses the CLI.
