# Roadmap

Scope: distribution, steps with status, open items, accepted behaviours. Back to [../design.md](../design.md).

## Distribution

Public repo, protected default branch, PRs with CI (fmt, clippy `-D warnings`, test). Each push
to the default branch builds Linux x86-64 and macOS binaries into a rolling `dev` release.
`release-plz` makes versioned releases; `install.sh` installs `dev` or a version.

## Steps

1. `measure` -> cache. Done. ([measure.md](measure.md))
2. `check`. Done. ([check.md](check.md))
3. `serve`: shows the same `Diagnostic` values as `check`; every file:line span opens the code
   viewer. The UX ports the Python prototype viewer. Done. Layout deviations: [serve.md](serve.md), "Layout port".
4. Cleanup of the old stack and `brickie llms.txt`. Done.
5. Publish: repo, CI, branch protection, dev release, install script, release-plz. Open.
6. Incremental measure. Open: re-measure complexity of changed files (tree-sitter only); mark
   dependencies stale when a diff touches `use`, `impl` or `pub` signatures (a full SCIP run
   is still needed); PR mode reports only findings new or in files changed against a base.
7. Nested bricks: per-level checks, `exports`, zoom. Done. ([levels.md](levels.md))
8. Brick interfaces. Done.
9. Versioned design editing. CLI and viewer done. ([editor.md](editor.md))
10. Code viewer. Done, except hover.
11. Open: an MCP server with notifications, so the agent need not poll `brickie design show`.
12. Open, optional: an id tie-break for `smart_route` top/bottom ports. Output is stable today;
    the tie-break would change golden offsets for equal keys.

## Accepted behaviours

Pinned by tests, documented in code: `pub(in ..)` and unknown visibility forms widen to
crate-wide on purpose; a symbol defined twice in one file keeps the first row; `smart_route` port
capacity is counted, never enforced.
