# Proposal: link states, link descriptions and scoped check

Status: agreed after two independent reviews. Core implemented (`brickie-check` `unused_links`,
`deps/unused`, `check --brick/--skip`, todo, `llms.txt`); the viewer part is implemented too (served `filter` per link, one line per pair, view mode, hover label). Related:
[design-collab.md](design-collab.md).

## Link states

Decision (supersedes the earlier planned/unused split): the `planned` state is removed because it
added complexity. Every allow rule between two named bricks with no backing edge is `unused`
(`deps/unused`, warning), whether or not its ends have code. The agent always sees the status and
hides it while coding with `--skip deps/unused`.

Computed once in `brickie-check` (`unused_links`) and served to `check`, the design todo list
(`unused`, text names `deps/unused`) and the viewer; nobody derives it again.

| State | Condition | Line | Mark | Todo | `check` |
| --- | --- | --- | --- | --- | --- |
| unused | allow rule, no backing edge | kind dash, warning colour | `!` | `unused`, one line per rule | `deps/unused` (warning) |
| no reason | any rule without a `reason` | unchanged | warning `?` | `design/no-reason` | none |

The warning colour is one token (`--warn`, the brick amber `--bd-amber`: `#a67c00` light,
`#f1d98a` dark) shared by bricks and links; the glyph tells the warning apart, so it reads without
colour.

- Only named brick-to-brick allow rules; deny rules and tag or `*` selectors are excluded.
- "Backing edge" keeps today's meaning (`report/edges.rs`): level path, edges entering a
  composite through a named child (`via`), the rule's `kinds` and `scope`. The `deps/unused`
  message names the filter, for example "no `uses` edge in test scope".
- Dash stays the edge kind (`implements`, `reexports`); colour and a glyph mark the state.
- The viewer draws one line per (from, to) pair and lists every rule in the tooltip, in view mode
  and in Design mode.
- `deps/unused` can be accepted in `brickie.review.toml`; its subject is
  `level + from + to + kinds + scope` (no file line), so the id survives edits of the toml.

## Link description

The rule `reason` shows as a label on hover or selection only, placed like the existing marks.
Touch: a tap selects and shows it; the inspector holds the full text.

## Scoped check

| Option | Meaning |
| --- | --- |
| `--brick PATH` (repeatable, path from the root, includes descendants) | evaluate the whole workspace; show findings whose owners meet the scope; findings without an owner always show; the exit code uses the shown set; a footer reads `outside scope: N errors, M warnings` |
| `--skip CODE` (repeatable or comma separated) | hide findings with these codes whose level is warning; an error finding with a skipped code still shows and counts; the output ends with `skipped: N warnings (codes)` |

Owners: source brick for `deps/deny`, `deps/default`, `deps/private`; every member for
`deps/cycle`; both ends for `deps/unused`; the defining brick for `interface/*`; the function's
brick for `complexity/*`. `stale/file`, `bricks/unmapped` have no owner; `config/*` still exits 2.
`[[accept]]` applies before the filter. `--skip` hides and never accepts.

Agent workflow in `llms.txt`: while coding brick X run
`brickie check --brick X --skip interface/unused,deps/unused`; before reporting done run a full
`brickie check`.

## Also

`llms.txt` "interface/unused" still suggests `pub(super)`; align it with the help text.

## Review

| Topic | Reviewer A (UX) | Reviewer B (check, agent) | Decision |
| --- | --- | --- | --- |
| Encoding | dash already means edge kind; use amber and `!` (opacity dropped with `planned`) | - | A |
| Partly measured end | unused only when fully measured | any measured descendant counts | superseded: no `planned` state; always unused, hide with `--skip` |
| Todo grouping per brick | cut; keep one line per rule | - | A |
| Where the logic lives | compute in core, serve to UI | move `backed` logic from `change` to `check`, shared | both |
| Finding id | - | no file line in the subject | B |
| `deps/*` owner | - | source only, match by level path | B |
| `--skip` | - | per finding by level; or `--errors-only` instead | `--skip` filtered by finding level: the user asked to hide unused warnings only |
| View mode | draw unused links there too | - | A |
