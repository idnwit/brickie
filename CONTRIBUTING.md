# Contributing

## Workflow

`main` is protected: open a pull request, keep it small, and wait for the `ci` check to pass.
Commits and PR titles must follow [Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<optional scope>): <summary>` with type one of `feat`, `fix`, `docs`, `refactor`,
`perf`, `test`, `build`, `ci`, `chore`, `revert`; add `!` or a `BREAKING CHANGE:` footer for
breaking changes. PRs are merged by squash or rebase (no merge commits; `main` keeps a linear
history). With squash the PR title becomes the commit on `main` (the `pr-title` check enforces
it); with rebase every commit must be a Conventional Commit. The changelog and version bump are
generated from these commits.

## Checks

Run before pushing (CI runs the same, plus a wasm build of the viewer):

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p brickie-ui --target wasm32-unknown-unknown --features hydrate -- -D warnings
cargo test --workspace        # the measure test needs rust-analyzer on PATH
```

Design notes are in [docs/design.md](docs/design.md); update them with behaviour changes.

## Releases

Every push to `main` refreshes the rolling `dev` prerelease. Versioned releases are automated by
[release-plz](https://release-plz.dev): after merges it opens a "release" PR that bumps the
version and updates `CHANGELOG.md`; merging it creates the `v<version>` tag and GitHub release,
which triggers the binary build and upload. Nothing is published to crates.io.

Maintainers: store a GitHub App or fine-grained PAT as the repository secret `RELEASE_PLZ_TOKEN`;
PRs and releases made with the default `GITHUB_TOKEN` do not trigger CI or the binary workflow.
