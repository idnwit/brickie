# UX principles

Scope: the interaction method every surface follows, in the viewer and, where it applies, in
`brickie design`. It names no widget, colour or key; the viewer's realisation is in
[../webui.md](../webui.md), which cites these principles by number. Back to [../design.md](../design.md).
Reasons for the editing model: [editor.md](editor.md), [serve.md](serve.md), [levels.md](levels.md).

| # | Principle | Reason |
| --- | --- | --- |
| P1 | Creating, connecting, moving, renaming and deleting design elements are gestures on the canvas, not forms elsewhere, and a connection picks its type first so nothing is left to fill in after the drop. | The canvas is the design, so the gesture and its result are in the same place. |
| P2 | Containment follows the data model: dropping a brick on a brick nests it. | The gesture then means exactly what the file tree does. |
| P3 | One selection at a time (a set of bricks counts as one) with one inspector; selecting scopes every list and summary to it, and clearing restores the whole view. | The user never asks which panel is current, and "what about this brick" needs no manual filtering. |
| P4 | Every cancelling or destructive gesture has an escape that backs out one step at a time. | One predictable key replaces hunting for a close button. |
| P5 | Every design edit is written at once as one version of the whole design, one gesture or agent task is one version, and there is no pending, review or publish step. | One truth for `check`, git and editors, and one gesture gives one Undo and one toast. |
| P6 | Every version is undoable and redoable as a new version, history is append-only, and Undo acts only on the current user's own latest versions. | Approval happens after the fact, no work is lost, and undo never silently reverts someone else. |
| P7 | Every version shows its author (agent, human or hand edit), the surface decides the author, and a change by another writer reaches the open view without a reload, says what it did and offers Undo. | An agent cannot pose as the human, and a reviewer sees who changed what at once. |
| P8 | A write names the version it is based on; on an old base it is re-sent only when no version in between touched the same element, otherwise it is refused naming the author and element, in one short line, leaving the design unchanged. | An edit by identity must not override another writer's change, and the user can correct a refusal without reading logs. |
| P9 | View preferences such as positions are not versions: they never go stale and are never undone. | A position is not part of the design. |
| P10 | An agent can see what others changed since its last version, can check a large change without writing, and the human can hand it a precise name of the selected element. | The chat is the only channel between them, and the agent need not remember what it missed. |
| P11 | The whole view state lives in the URL with real link targets, so back and forward restore the view and a shared link shows the same view; reload, live updates and re-measure keep view, selection and pan and zoom and never interrupt a gesture or typing. | Users never lose place while updates arrive. |
| P12 | Design versus reality stays legible: planned elements, planned links (declared, no code edge yet), denied links, drift and agent-made changes each look different from measured, healthy, human-made ones, and no state is encoded by colour alone. | The tool exists to show the gap between intent and code, including to colour-blind users and in monochrome print. |
| P13 | Changing state never moves or resizes layout, layout is computed once per level, and detail that clutters appears on hover, selection or in the inspector. | Moving targets cost clicks and lose place, and progressive disclosure keeps the overview readable. |
| P14 | Nested structure is shown one level at a time with a visible way in and out and de-emphasised neighbours; what a level hides still shows where it matters, so a connection not allowed to cross is drawn up to the boundary and marked stopped. | A missing line must not read as "no dependency", and each view stays small while context stays. |
| P15 | The default view is the lowest-noise one; detail is added by presets and layers, and the URL records the difference. | Progressive disclosure with reproducible views. |
| P16 | Scores and status come from one computation shared by CLI and viewer. | The viewer never disagrees with `check`. |
| P17 | A complete read-only view renders without JavaScript, and reading source is one step from any finding in a read-only viewer that does not change the view. | Review works in any environment and a finding is verified in code without leaving the page. |
| P18 | Every pointer action is keyboard-reachable except freehand positioning, focus is visible, single-key shortcuts never fire while typing and are listed in an in-app sheet, contrast meets WCAG 2.1 AA in light and dark following the system theme, and motion honours the reduced-motion preference. | Keyboard, assistive-technology, low-vision and vestibular-sensitive users, and typing must not trigger commands. |
| P19 | Writing endpoints accept only same-origin requests from the local machine. | A web page open in the same browser must not edit the design. |
