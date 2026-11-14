# Design editing

Scope: how the design is edited and versioned (`brickie design` and the viewer's Design mode). Back to
[../design.md](../design.md).

The `brickie.toml` tree is the design. Every edit is applied and written to the files at once, as
one new version of the whole tree. A human (viewer) and a coding agent (CLI) co-edit it directly;
there is no change session, review or publish step.

## Decisions

| Topic | Decision | Reason |
| --- | --- | --- |
| Source of truth | the files; no pending op log | one truth that `check`, git and editors all see |
| Version | one integer `N` for the whole tree (`v15`); internally a hash (FNV-1a 64) of every config file, sorted path and bytes | simple to quote in prompts and errors; the hash detects any edit |
| History | `.brickie/history/` (git-ignored): `log.jsonl` with `{v, hash, prev, by, at, note, ops, diff, files, removed, full}`, `v<N>/` with the files version N wrote; last 200 kept, the oldest then rewritten as a full snapshot | undo and diff without git; bounded size |
| Author | `by` follows the writer: CLI `agent`, viewer `human`, a hand edit `external`; no flag or env | an agent cannot pose as the human |
| Hand edits | on every read or write, a tree hash different from the head's is first recorded as an `external` version | hand edits stay visible and make old bases stale |
| Concurrency | every write takes `--base N` (env `BRICKIE_BASE`); `N` not the head: nothing written, exit 3, `design/stale` with the changes since `N` per author | optimistic and explicit; no hidden reader cursor |
| Atomic write | one function `Design::apply(base, by, note, ops)` under `.brickie/design.lock`: sync hand edits, check base, apply ops in memory (`toml_edit`, comments kept), load and validate the whole design, write each changed file by temp file and rename, append the version | an invalid design is never written; CLI and server share it |
| Identity | brick by name; rule by `(file, from, to, kinds, scope)`; complexity rule by `(file, bricks)`; never by index | a stale index silently edited another rule before |
| Missing or ambiguous key | `design/no-match` listing candidates; ambiguous asks for `--kind` / `--scope`; adding an existing identity is `design/exists` | an edit must name exactly one element |
| `set_rule`, `set_complexity` | patches: only given fields change | concurrent edits of different fields do not clobber |
| Undo | `undo [V]` restores the bytes the version replaced (comments included) as a new version; refused when a file it changed is no longer as V left it (a later version changed it and was not reverted); an agent may not undo a human or external version, except an external one that only deleted files (it restores them); `show` loads a missing nested file as an empty level with a warning | history stays append-only; no lost human work; step-by-step undo works once the later versions are reverted |
| One task, one version | `brickie design apply --base N --note` reads one op per stdin line (the write subcommands' syntax without the prefix and without `--base`, `--note`, `--dry-run`; clap re-parses each line), builds every op against the design on disk and writes them through one `Design::apply`; a bad line aborts with its number | one agent task is one toast and one Undo; the line syntax is the one already documented |
| Dry run | `--dry-run` on `apply` and every write: `Design::dry_run` runs the same checks in memory and returns the diff, the todos the change adds and the scaffold plan, headed `dry run on v<head>` so the agent uses the right base; nothing is written | an agent can check a large change first; no UI |
| Since agent | `design show` starts with the versions after the latest `agent` one (nothing when none, or when no agent version exists); JSON `since_agent` | the agent need not remember what it missed |
| Smaller show | `design show --in P` prints the level owned by composite P and its subtree | cheaper reads of a large design |
| Scaffold | `brick set --path` creates a missing `<path>/brickie.toml`; a write lists each new nested folder (`created dom/ (group)`); `add_brick` scaffolds missing code (crate, module file, workspace member); never overwrites; undo does not remove code | design and code start together; code is the agent's |
| Ungroup | the child file leaves the tree but stays on disk | never delete hand-written files |
| Codes | removed `change/*`; `design/planned`, `no-reason`, `invalid`, `stale`, `no-match`, `exists` | no change sessions remain |

Reviewed by two independent reviewers before implementation; both preferred direct versioned
editing over the propose/accept/publish session (fewer states, no publish conflicts, hand edits
handled).

## CLI

`brickie design show [--since N] [--in P] | log | diff | undo | apply | brick | rule | export | limit | deps | include`; see
`brickie design --help` and "Editing the design" in [../llms.txt](../llms.txt). Code:
`crates/core/change` (`design/` versions, history, lock, writes; `edit/` ops; `report/` diff,
show, todo) and `crates/cli/src/cli/design/`.

## Viewer (Design mode)

Code: `crates/web/server/src/routes/editor.rs`, `data/design/`, `events/scan.rs`;
`crates/web/api/src/{model,endpoint}/editor.rs`; `crates/web/ui/src/{state,components}/editor/`.
User guide: [../webui.md](../webui.md#design-mode).

| Topic | Decision | Reason |
| --- | --- | --- |
| Endpoints | `GET /api/design` (`DesignState`: head `v`, `by`, last 50 versions, todo, level files and selectors, per-file sheets, layout hints); `POST /api/design/{edit, move, undo}` (one human version each, `base` required); `links`, `link-check`, `levels` (read only); `layout`, `relayout` (hints) | one current path over `brickie_change::Design`, shared with the CLI |
| Author | every viewer write is `human`; no field in the request | the surface decides, as in the CLI |
| Stale base | 409 with the `design/stale` text and header `x-brickie-head`; the page reloads the design and re-sends the same request once on the new head only when no version since its base touched an identity of the request (brick by name, rule by file and key, limit by file and `bricks`; a rule op also touches the bricks it names `from`/`to`, so a rename or removal of one stales it; a version without ops touches all); the server checks this and answers a clash with `<author> v<N> changed <identity>` (no stale text), shown as `Not applied: <that>` | an edit by identity must not override another writer's change to the same thing |
| Copy ref | an icon button in the brick and rule inspector copies the CLI name (`web/api`, `rule web -> store (prod)`; `[kinds]` only when another rule of the file has the same ends and scope); tooltip "Copy ref for the agent" | the human pastes it into the agent's chat |
| Identity in the UI | a selected rule is `(file, RuleKey)`; editing kinds, scope or target is `remove_rule` + `add_rule` in one version, other fields a `set_rule` patch; complexity rows by `bricks` | no index can point at another rule after a concurrent edit |
| Gestures | the ops of one gesture (a tile into a leaf brick: `make_composite` + `add_brick`) are one version | one `Ctrl+Z` per gesture |
| Undo / Redo | undo reverts the human's latest edit (`undo vN`); redo reverts that undo; both stacks are read from the history | survives reloads; skips other writers' versions |
| Live updates | the watcher reads the head when `.brickie/history/log.jsonl` or the toml tree changes (a hand edit is then recorded as `external`) and sends `design {hash, v, by}`; a page whose `v` differs fetches the design and levels and patches in place; another writer's version shows a toast `agent v16: <note> · Undo` when the version has a note, else "agent changed: <diff line>" with Undo | agent and hand edits are visible at once and reversible |
| Layout hints | `.brickie/layout.json` (`{"root/web/api": {x, y}}`), beside the history, not versioned, not in `brickie.toml` | a position is a view preference: no version, never stale, not undone |
| Auto layout | always enabled; in Design mode it clears the hints of the level on screen, lays it out again and fits; a brick without a hint that would overlap a placed one goes right of the boxes | the button did nothing while hints won over the layout |
| New tag lane | the drop zone and the frame of a drag leave the dragged brick out; fitting in Design mode keeps room for the zone under the frame | the zone used to follow the dragged brick down and could not be reached |
| Toolbar | one row of groups with dividers, `nowrap`; labels hide below 1280 px (glyph and tooltip stay), then it scrolls sideways | it wrapped into three rows over the canvas |

## Status

Phase 1 (engine and CLI) and phase 2 (viewer on the versioned design) are implemented and tested
(server tests for edit, undo, stale 409, layout and events; UI unit tests for the retry, undo
stacks, hit tests, new-lane zone, layout hints and op building). Not checked in a browser by
automated tests: the toolbar's look at narrow widths and drag gestures end to end.

## UX principles (viewer)

Canvas first (create, connect, move, delete, rename are gestures); one selection, one inspector;
every edit is a version with Undo; never lose place; design vs reality stays legible (planned bricks
dashed, unused links and drift amber, agent versions marked). Connecting is type first: one port per link type,
nothing to fill in after the drop. A brick is a folder: dropping a brick on a brick nests it.
