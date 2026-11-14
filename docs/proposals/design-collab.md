# Proposal: human and agent collaboration on the design

Status: agreed after two independent reviews; items 1 to 5 are implemented and tested. Not yet
checked in a browser: the Copy ref button and its clipboard write. Viewer undo still retries on a
new head without the conflict check. Context:
[../design/editor.md](../design/editor.md).

The agent is a coding CLI (Codex, Claude Code) that runs only when the human prompts it in that
CLI's chat. The human explains intent in the chat and edits the design in the viewer. brickie adds
no conversation channel; it connects the two surfaces with as little UI as possible. Every edit
stays live at once, and approval is after the fact with Undo.

## Agreed set

| # | Item | Form | Surface |
| --- | --- | --- | --- |
| 1 | Viewer retry does not override | the automatic re-send on a new head happens only when no version in between touched an identity of the request; otherwise `Not applied: <reason>` | viewer (bug fix) |
| 2 | One agent task, one version | `brickie design apply --base N --note "..."` reads one design op per stdin line and writes them through the existing `Design::apply(.., ops)` as one version; `--dry-run` on `apply` and every write prints the diff and new todos and writes nothing; the toast reads `agent v16: <note> · Undo` | CLI, toast |
| 3 | Changes since the agent's last version | `brickie design show` starts with the human and external versions after the latest `agent` version (author, note, diff lines) when there are any; no flag. `llms.txt`: run `show` at the start of every design task and never reuse a remembered base | CLI, `llms.txt` |
| 4 | Copy ref | one icon in the inspector copies the CLI name: `web/api` or `rule web -> store (prod)` | viewer |
| 5 | Smaller show | `brickie design show --in P` | CLI |

## Rejected

| Item | Reason |
| --- | --- |
| Question or request thread in brickie | duplicates the CLI chat |
| `wait` or push notifications | the agent runs only when prompted; item 3 covers what it missed |
| Note box on viewer gestures | slows every edit; the human explains in the chat. "Add note to vN" in History stays a possible later step |
| Viewer preview of a dry-run, "Undo all" | one version per task gives one Undo; the dry-run text goes to the chat |
| Lock on bricks or rules | schema, code and UI for what "do not touch X" in the chat already says |
| Todos closed by measure | the task chips already shrink |
| `show --depth` | `--in` is enough; fewer options for an agent to misuse |
| Canvas highlight of agent changes | deferred: first check that the existing agent marks and toast are not enough |

## Review

Reviewer 1 (designer UX) and reviewer 2 (agent workflow, code) agreed on items 1, 2 (batch), 4,
5 and on cutting the lock, measure toast and `--depth`. Reconciled differences:

- Dry-run: reviewer 1 cut it, reviewer 2 kept it on the CLI only. Kept on the CLI: `apply`
  already validates in memory, and it adds no UI.
- Since-agent: a flag (reviewer 2) or the default header (reviewer 1). Default header: nothing for
  the agent to remember.
- `@vN` in the copied ref: dropped; the agent always reads the head.
- Human note: cut (reviewer 1) or note on an existing version (reviewer 2). Cut for now.
- Highlight: outline of the latest agent version (reviewer 1) or cut (reviewer 2). Deferred.
