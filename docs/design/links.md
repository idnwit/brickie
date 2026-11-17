# Rules and links

Scope: how dependencies between bricks are allowed. Back to [../design.md](../design.md). The
rules themselves are stated once in `docs/llms.txt` ("Rules"); this file records why.

## Decision

- A `[[rule]]` is an allow-only principle: `from`, `to`, `kinds`, `scope`, required `reason`.
  Nothing is allowed unless a rule allows it. There is no deny, no action, no specificity score,
  no tie to break, no `via`.
- A `[[link]]` is one explicitly declared dependency: `from`, `to` (a sibling brick or
  `composite/child` for an exported child), `kinds`, `scope`, required `reason`.
- A link is valid iff every kind and scope it names is allowed by some rule of its level. An
  invalid link may sit in a draft but blocks `design publish` (`design/link-denied`; design
  files only, no code is read).
- A measured edge is allowed iff a valid link covers it; otherwise `deps/unlinked`. Exports
  stay a separate check (`deps/private`).

| Reason | Effect |
| --- | --- |
| Each dependency is an explicit line with its own reason | a reviewer reads edges one by one and the diff of a design change is the list of new links |
| Rules are principles, links are instances | the few rules stay readable; agents add links, a human owns rules |
| No specificity scoring and no deny | a decision is local to one rule and one link; nothing changes because a rule elsewhere got more specific |
| `composite/child` replaces `via` | the entered child is part of the link, so a port-style dependency is one line; `via` made a rule decide on a property of the edge |
| Design-only judgement of links | a drawn design can be judged before any code exists; Design mode shows a denied link in red |

Removed with this decision: `deps/deny`, `config/unknown-via`, `design/no-reason` (reasons are
required by the schema), unused-rule findings (a valid link with no edge is information, the
planned link, a todo of `check`), and the root-only limit on `independent`.

## Identity

Rules and links are identified by `(file, from, to, kinds as a set, scope)`, an absent scope
meaning `all`. `rm` and `set` address an element by that key; `--kind` and `--scope` narrow
several candidates. Reason: stable across reordering and concurrent edits, never an index
([editor.md](editor.md)). A link has one `to`; a second target is a second link.

A bad link is `config/invalid`: unknown brick, link to itself, `composite/child` on a brick that
is not a composite, or a child that does not exist or is not in `exports` (no grandchildren).

## Independent bricks

`independent = true` is allowed at any level. Reason: a library serves the bricks around it,
not the application above.

- Inside its parent's subtree (its siblings and anything nested in them) edges into it need no
  rule and no link.
- From outside its parent composite it is reached like any exported child: the outer level's
  rules, links and exports apply. Exporting it is allowed but discouraged; move it up to the
  level that needs it instead (design advice, shown next to the rule).
- Edges out of it need a rule and a link and may go only to another independent brick of the
  same level. A rule or link from an independent to a normal brick is never valid: the link is
  denied and the code edge is `deps/unlinked`.
- `*` never matches an independent brick.

## Cycles

Cycles among a level's links are information only (a todo of `check`,
marked in Design mode): a plan may be redrawn. Cycles in measured code stay
`deps/cycle` errors. Every check finding is still an error.

## Shared decision functions

`rule_allows`, `judge_link`, `covering_link`, `free` and `link_cycles` in
`crates/core/check/src/config/decide.rs` answer each question once for `check`, the publish
gate, the design editor and the viewer ([check.md](check.md)).

## Open questions for the user

| Question | Chosen reading |
| --- | --- |
| Do edges from an independent brick to another independent brick need a rule and a link? | yes, both (outbound edges are always explicit) |
| Should `link set` change more than `reason`? | no: only `reason`; changing ends, kinds or scope is remove plus add, as for rules |
