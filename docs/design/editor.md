# Design editor

Scope: edit mode of the viewer, change sessions, `brickie change`. Back to [../design.md](../design.md).

Edit mode of the viewer over a **change**: a pending, ordered log of ops over the `brickie.toml`
tree. One human and one coding agent share it.

## Decisions

- The `brickie.toml` tree stays the source of truth. Nobody writes it while a change is open.
  Files change only on publish.
- Change tracking is independent of git. One change, one human, one agent.
- Code edits are out of scope. The agent edits code as usual and only proposes config ops.
- **One engine**: the loader takes a file-source trait. Preview replays ops over the stored
  `base/` documents in memory and runs the normal loader and `check`. Publish writes the same
  documents, keeps comments, and scaffolds folders or crate files. It never moves or deletes code.
- Storage `.brickie/change/` (git-ignored): `base/`, append-only `ops.json`, `intent`,
  `readers.json`, `lock`, `layout.json` (canvas positions, not ops).
- An op records `{seq, by, at, op, file, key, before, after, note, state, review}`. `before`
  must match the file; otherwise the op is a `conflict`, derived on every read. Human ops are
  accepted when recorded; agent ops are `proposed` until a human reviews. Reviews append
  entries; nothing is rewritten.
- Publish refuses on a proposed or conflicting op, a touched file edited outside brickie, or a
  result that fails `load`.
- **Planned / drift**: planned = a brick or allowed edge with no measured code; drift = `deps/*`
  findings on preview edges. A change is built when neither exists and there are no errors.
- **Draft preview**: partition errors are collected instead of aborting, so the canvas updates
  mid-design. Each is a todo task and a `change/invalid` publish blocker. `check` stays strict.
- A write whose preview fails hard is still recorded; the reply carries `change.error`.
- Every write answers `ChangeReply {done, change, levels}`; the page patches in place and keeps
  pan, zoom and selection.
- CLI: `brickie change` (see `--help` and the "Change sessions" section of [../llms.txt](../llms.txt)).

| Topic | Decision | Reason |
| --- | --- | --- |
| Rename / move vs accepted review ids | not rewritten; old acceptance stops matching | an id hashes the finding, which changes with the name |
| Rules without `reason` | `change/no-reason` todo and publish warning, not a blocker | drawing a link must not block |
| Retract and redo | review entries (`retract`, `restore`); redo fails if the op conflicts a later one | the log stays append-only |
| Ungroup | refused when the child file has `include` or `[deps] default/cycles` | cannot merge safely |
| Ops from the UI | recorded as the human only; the agent uses the CLI | one writer per role |
| Not offered | reordering (no semantics); removing a measured code edge | that is a code change |

## UX principles

Canvas first (create, connect, move, delete, rename are gestures); one selection, one inspector;
every edit is visibly an op with Undo; never lose place; design vs reality stays legible (planned
dashed, drift amber, agent proposal violet with `A`, conflict red with `!`). Connecting is type
first: one port per link type, nothing to fill in after the drop. A brick is a folder: dropping a
brick on a brick nests it. A tag lane drop retags the first tag.

Known gaps: draggable toolbar, drag-a-box brick sizing, autocomplete for members, overlay diff.
