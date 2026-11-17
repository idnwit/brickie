# Design editing

Scope: how the design is edited and versioned (`brickie design` and the viewer's Design mode). Back to
[../design.md](../design.md).

The design has two trees. The published tree is the real `brickie.toml` tree that `check`, the
gate and the viewer's normal mode read. The draft (`.brickie/draft/`, one at a time) is what
every design edit writes. `brickie design publish` copies the draft into the real files once the
code matches the draft's structure.

## Draft and publish

The earlier decision "no change session, review or publish step" is reversed. Reason: a design
drawn ahead of the code broke the published tree that `check` gates on (unassigned modules,
planned bricks) and needed a tolerant `Mode::Draft` resolve everywhere. Publish is not an
approval step: it waits only for the code to match the drawn target, then the target becomes
the gated design. User-approved; reviewed by two independent reviewers.

Two rules of the viewer's Design mode (user decision):

1. Design mode shows only what the design files say: the draft when one exists, else the
   published design. No measured code: no measured edges, scores, metrics, findings, module
   lists or counts, code panels, measure controls or notices, and no planned or unmapped
   markers. Members show as declared (selectors as text). Reason: the draft is a target drawn
   ahead of the code; mixing in code made the canvas show a state that is neither.
2. Code is compared with the design only by the publish gate, when Publish is clicked (a dry
   run or the real publish). Reason: one place decides "the code now has this structure".

| Topic | Decision | Reason |
| --- | --- | --- |
| Two trees | published = the real files, read by `check`, the gate and the viewer's normal mode, which never read `.brickie/draft/`; draft = the target being drawn | the gate stays green while a refactor is planned |
| Where edits go | every `brickie design` write, `apply` and the undo of a draft version write the draft; no flag writes the published tree; the first write forks the draft from the published head (a write that changes nothing creates none) | one path; the published tree changes only by `publish`, by hand (`external`) and by undoing a published version |
| Storage | `.brickie/draft/draft.json` (`{forked_from}`) and `files/` mirroring the relative config paths (`..` as `^`); read through the `Preview` overlay, so `path =` and includes resolve as for real files | a `Files` snapshot, no op log to replay on read |
| History | one shared version counter; each version has `track` (`draft` or `published`); a fork or rebase version carries `forked_from` and a full snapshot; an undo carries `reverts`; states replay their own track only; compaction keeps each track's head and makes the first kept version of each track full | one log, one undo model; `--base` and stale checks use the head of the track written |
| Free-style draft | a draft write validates config syntax and self-consistency only (`load_config`), never against the code; `show` todo lists design-file items only (`link_denied`, `link_cycle`); headers name the track (`draft v42 from published v37`) | the target may be far from the code |
| Publish gate | `design publish --base N` strictly resolves the draft over the cache on disk: blocked (`design/blocked`, exit 4) by `config/unassigned`, `config/overlap`, `config/member-outside-parent`, `design/planned`, `design/link-denied` (read from the design files alone), a missing, mismatched or stale cache; dependency findings never block; no partial publish | publish means "the code now has this structure" |
| Checklist | `design check` runs the gate on the working design (draft, else published) and lists every blocking item (JSON: `code, message, file, brick, member, module`) plus `info` (dependency findings, planned links, links on a cycle); exit 4 `design/blocked` when blocked; the viewer's `publish` with `dry_run` calls the same `Design::publish_check` gate | the agent's refactor checklist |
| Code reading | only `design check` and `design publish` read the code; `show` and every other command read design files alone (`Design::show_design`, no cache needed) and list config todos only | one rule, same as the viewer's Design mode |
| Publish write | one published version (author: who runs it, note `publish draft vN`), files left only by ungroup stay on disk, the draft is removed; undoable by restoring bytes like any published version | same write path as before |
| Stale publish | published head != `forked_from`: `design/stale` listing the published versions since the fork; `design rebase --base N` replays the live ops of the draft (undone versions skipped) onto the new head as one draft version, failing on a clash through the ops' own identity errors; no TOML merge | no silent merge |
| Discard | `design discard --base N` removes the draft and appends a draft-track marker | history stays append-only |
| Scaffold | allowed on draft writes: writing code for draft bricks is the intended refactor path; `check` reports that code as `bricks/unmapped` until the publish | the code can move toward the target before it is published |
| Resolve | one strict resolve; it collects every partition error and fails with a `Partition` listing them all; `Mode::Draft`, `Pending` and `load_draft_from` are gone | one path; the checklist needs every item |

## Decisions

| Topic | Decision | Reason |
| --- | --- | --- |
| Source of truth | the published files and the draft snapshot; no pending op log | `check`, git and editors see the published files |
| Version | one integer `N` for the whole tree (`v15`); internally a hash (FNV-1a 64) of every config file, sorted path and bytes | simple to quote in prompts and errors; the hash detects any edit |
| History | `.brickie/history/` (git-ignored): `log.jsonl` with `{v, track, forked_from, reverts, hash, prev, by, at, note, ops, diff, files, removed, full}`, `v<N>/` with the files version N wrote; last 200 kept plus each track's head, the first kept of each track rewritten as a full snapshot | undo and diff without git; bounded size |
| Author | `by` follows the writer: CLI `agent`, viewer `human`, a hand edit `external`; no flag or env | an agent cannot pose as the human |
| Hand edits | on every read or write, a published tree hash different from the published head's is first recorded as an `external` published version | hand edits stay visible and make a draft's publish stale |
| Concurrency | every write takes `--base N` (env `BRICKIE_BASE`), the head of the track written (the draft head; the published head when the write forks); `N` not that head: nothing written, exit 3, `design/stale` with the changes since `N` per author | optimistic and explicit; no hidden reader cursor |
| Atomic write | one function `Design::apply(base, by, note, ops)` under `.brickie/design.lock`: sync hand edits, check base, apply ops to the draft in memory (`toml_edit`, comments kept), load and validate the whole design, store the draft (temp directory and rename), append the version | an invalid design is never written; CLI and server share it |
| Identity | brick by name; rule and link by `(file, from, to, kinds as a set, scope; absent scope = all)`; complexity rule by `(file, bricks)`; never by index | a stale index silently edited another rule before |
| Missing or ambiguous key | `design/no-match` listing candidates; ambiguous asks for `--kind` / `--scope`; adding an existing identity is `design/exists` | an edit must name exactly one element |
| `set_rule`, `set_complexity` | patches: only given fields change | concurrent edits of different fields do not clobber |
| Undo | `undo [V]` (default: the working head) restores the bytes the version replaced (comments included) as a new version of V's track (the fork's "before" is the published version it forked from); refused when a file it changed is no longer as V left it (a later version changed it and was not reverted); an agent may not undo a human or external version, except an external one that only deleted files (it restores them); `show` loads a missing nested file as an empty level with a warning | history stays append-only; no lost human work; step-by-step undo works once the later versions are reverted |
| One task, one version | `brickie design apply --base N --note` reads one op per stdin line (the write subcommands' syntax without the prefix and without `--base`, `--note`, `--dry-run`; clap re-parses each line), builds every op against the design on disk and writes them through one `Design::apply`; a bad line aborts with its number | one agent task is one toast and one Undo; the line syntax is the one already documented |
| Dry run | `--dry-run` on `apply` and every write: `Design::dry_run` runs the same checks in memory and returns the diff, the todos the change adds, the notes and (with `--scaffold`) the code plan, headed `dry run on v<head>` so the agent uses the right base; nothing is written | an agent can check a large change first; no UI |
| Since agent | `design show` starts with the versions after the latest `agent` one (nothing when none, or when no agent version exists); JSON `since_agent` | the agent need not remember what it missed |
| Smaller show | `design show --in P` prints the level owned by composite P and its subtree | cheaper reads of a large design |
| Scaffold | `brick set --path` and `--kind group` create the nested `brickie.toml` (design files, always); a write lists each new nested folder (`created dom/ (group)`). Code is written only with `--scaffold` on `brick add` or `apply` (crate, module file, workspace member, `pub mod`); never overwrites; undo does not remove code. The web editor never scaffolds | a checker does not write source unless asked; a brick without code is a `planned` todo, the cue to write it |
| New level | a nested `brickie.toml` brickie creates is empty | nothing is allowed without a rule, so an empty file is already strict |
| Rule note | `link add` prints `note: design/link-denied: ...` when no rule of the level allows the link (`judge_link`, design files only); the write still succeeds | the agent learns at once that a human must add a rule |
| Ungroup | the child file leaves the tree but stays on disk | never delete hand-written files |
| Codes | `design/planned`, `link-denied`, `invalid`, `stale`, `no-match`, `exists`, `blocked` | |

## CLI

`brickie design show [--since N] [--in P] | log | diff | undo | apply | check | publish | rebase | discard | brick | rule | link | export | limit | include`; see
`brickie design --help` and "Editing the design" in [../llms.txt](../llms.txt). Code:
`crates/core/change` (`design/` versions, history, lock, writes, `design/draft/` draft store,
publish gate, rebase and discard; `edit/` ops; `report/` diff, show, todo) and `crates/cli/src/cli/design/`.

## Viewer (Design mode)

Code: `crates/web/server/src/routes/editor.rs`, `data/design/`, `events/scan.rs`;
`crates/web/api/src/{model,endpoint}/editor.rs`; `crates/web/ui/src/{state,components}/editor/`.
User guide: [../webui.md](../webui.md#design-mode).

| Topic | Decision | Reason |
| --- | --- | --- |
| Endpoints | `GET /api/design` (`DesignState`: working head `v`, `published` head, `forked_from` of the draft, `by`, last 50 versions, todo, level files and selectors, per-file sheets, layout hints); `POST /api/design/{edit, move, undo}` (one human draft version each, `base` required); `POST /api/design/{publish, rebase, discard}` (`base` = draft head; `publish` takes `dry_run` and `note`; a blocked publish or dry run is 409 `design/blocked` listing every blocking item); `links`, `link-check`, `levels` (read only); `layout`, `relayout` (hints) | one current path over `brickie_change::Design`, shared with the CLI |
| Draft in the viewer | Design mode (`edit=1`) reads the working tree from its files alone (`Design::load_working`, `Design::show_design`: no cache read, so it works before any measure); its levels carry the bricks, the rules (Rules tab) and the links (drawn on the canvas by kind colour; denied red with the reason in the tooltip; cycle-marked); the todo lists design-file items only; nothing runs the gate until Publish ([rules](#draft-and-publish)); view mode loads the published files and the cache; publish, rebase and discard call the engine and are never re-sent on a newer head | the gate and the rules stay in the engine; an action on a changed draft must be seen first |
| Author | every viewer write is `human`; no field in the request | the surface decides, as in the CLI |
| Stale base | 409 with the `design/stale` text and header `x-brickie-head`; the page reloads the design and re-sends the same request once on the new head only when no version since its base touched an identity of the request (brick by name, rule or link by file and key, limit by file and `bricks`; a rule op also touches the bricks it names `from`/`to`, so a rename or removal of one stales it; a version without ops touches all); the server checks this and answers a clash with `<author> v<N> changed <identity>` (no stale text), shown as `Not applied: <that>` | an edit by identity must not override another writer's change to the same thing |
| Independent | `set_brick.independent` (`brick set --independent true|false`) writes or removes `independent = true`; allowed at any level. `design show` marks it (`independent` in text, `"independent": true` in JSON); the web inspector shows an "independent lib" checkbox on every brick | one rule at every level; semantics in [links.md](links.md#independent-bricks) |
| Copy ref | an icon button in the brick and rule inspector copies the CLI name (`web/api`, `rule web -> store`; `[kinds]` and the scope only when another rule of the file has the same ends); tooltip "Copy ref for the agent" | the human pastes it into the agent's chat |
| Identity in the UI | a selected rule or link is `(file, key)`; editing kinds, scope or target is a remove plus an add in one version (`remove_link` + `add_link`), the reason a `set_link` / `set_rule` patch; dragging a port creates a link (`add_link`); complexity rows by `bricks` | no index can point at another rule after a concurrent edit |
| Gestures | the ops of one gesture (a tile into a leaf brick: `make_composite` + `add_brick`) are one version | one `Ctrl+Z` per gesture |
| Undo / Redo | undo reverts the human's latest edit (`undo vN`); redo reverts that undo; both stacks are read from the history | survives reloads; skips other writers' versions |
| Live updates | the watcher reads the head when `.brickie/history/log.jsonl` or the toml tree changes (a hand edit is then recorded as `external`) and sends `design {hash, v, by}`; a page whose `v` differs fetches the design and levels and patches in place; another writer's version shows a toast `agent v16: <note> · Undo` when the version has a note, else "agent changed: <diff line>" with Undo | agent and hand edits are visible at once and reversible |
| Layout hints | `.brickie/layout.json` (`{"root/web/api": {x, y}}`), beside the history, not versioned, not in `brickie.toml` | a position is a view preference: no version, never stale, not undone |
| Auto layout | always enabled; in Design mode it clears the hints of the level on screen, lays it out again and fits; a brick without a hint that would overlap a placed one goes right of the boxes | the button did nothing while hints won over the layout |
| New tag lane | the drop zone and the frame of a drag leave the dragged brick out; fitting in Design mode keeps room for the zone under the frame | the zone used to follow the dragged brick down and could not be reached |
| Toolbar | one row of groups with dividers, `nowrap`; labels hide below 1280 px (glyph and tooltip stay), then it scrolls sideways | it wrapped into three rows over the canvas |

## Status

Draft and publish: engine and CLI implemented and tested (`crates/core/change/src/tests/{draft,publish}.rs`,
`crates/cli/tests/design.rs`) and in the viewer (`crates/web/server/src/tests/{editor,draft,links}.rs`).

Phase 1 (engine and CLI) and phase 2 (viewer on the versioned design) are implemented and tested
(server tests for edit, undo, stale 409, layout and events; UI unit tests for the retry, undo
stacks, hit tests, new-lane zone, layout hints and op building). Not checked in a browser by
automated tests: the toolbar's look at narrow widths and drag gestures end to end.

## UX principles (viewer)

Canvas first (create, connect, move, delete, rename are gestures); one selection, one inspector;
every edit is a version with Undo; never lose place; design vs reality stays legible in view mode (planned links
and drift amber); agent versions marked. Connecting is type first: one port per link type,
nothing to fill in after the drop. A brick is a folder: dropping a brick on a brick nests it.
