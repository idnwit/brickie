# brickie: project instructions

brickie measures Rust workspaces (dependency edges, function complexity) and checks them against
rules in `brickie.toml`; `view` is a reviewer webview and `llms.txt` a guide for coding agents.
Read `README.md` and `docs/design.md` before architectural changes. `docs/design.md` records
agreed requirements, decisions and status; keep it in step with the code.

## Coordination

- The main agent is the user's collaborator and orchestrates: it plans, splits work, briefs and
  reviews subagents, and integrates and verifies. Bounded implementation, investigation and
  independent review go to subagents; pick the smallest model that fits (haiku: mechanical edits,
  searches, running checks; sonnet: bounded implementation; opus: hard design or review).
- Give each subagent explicit ownership, deliverables and acceptance checks so edits do not
  overlap. Pass user steering on, and carry new hints into ongoing work.
- A subagent's completion message is evidence to inspect, not proof. The main agent checks the
  integrated result and reports its verification status.
- For significant design or correctness questions consult two independent reviewers, reconcile,
  and record the decision and evidence in `docs/`.
- Keep user-facing discussion short. Distinguish proposals from implemented and verified
  behaviour; never report something as done without code and verification.

## Token economy

Adapted from the caveman skill; full rules in [.rules/caveman.md](.rules/caveman.md). Every token is
billed and read.

- Chat replies: answer first, then reason, then next step. No greeting, hedging, recap or closer.
  Short words, one idea per sentence. Drop articles only when the sentence still reads in one pass;
  never drop `not`/`never`/`no`/`only`. Reply in the user's language.
- Code, commands, paths, numbers and errors stay verbatim. Quote only the shortest decisive error
  line. Show changed lines plus 1-2 lines of context, not whole files.
- No text between routine tool calls. One line before a multi-step run, one per phase change, one
  with the result.
- Read only what is needed: search first, then read a line range. Do not re-read a file just edited.
  Send searches, sweeps and check runs to a small subagent and keep only its conclusion.
- Use plain full sentences for anything persisted (code, comments, docs, commits, PRs), for
  security warnings, irreversible actions, and questions to the user.
- Docs: state each fact once and link instead of repeating. Prefer tables and lists over prose.
  Drop history, narration and status that git already holds; keep decisions and their reasons.

## Code and verification

- Rust checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  (plus `cargo clippy -p brickie-ui --features hydrate --target wasm32-unknown-unknown` when the
  UI changed), `cargo test --workspace`. Documentation-only changes need a consistency and link
  review.
- Source layout is in `docs/design/layout.md`: folders by concern, small focused files
  (about 300 lines or fewer, tests aside), no flat `src/`; crates by layer under `crates/`.
- Development stage: no backward compatibility. Keep a single current API/model path; breaking
  CLI or schema changes are fine, but update callers, docs and tests in the same change.
- `docs/llms.txt` is embedded in the binary and read by agents with no repo access: keep it
  standalone, with no links or references to other repo files.
- Adding a diagnostic code: add it to `crates/core/model/src/codes.rs` and document how to fix it in
  `docs/llms.txt` (a test enforces this).
- The viewer bundle is built with `scripts/build-ui.sh`; `assets/ui/` is git-ignored.
- Never commit private or machine-specific names, paths or addresses (test fixtures, docs,
  comments, snapshots included).

## Git and shared working tree

- Never commit or push unless the user explicitly asks. Commit messages and PR titles follow
  Conventional Commits (see CONTRIBUTING.md).
- Parallel agents share one working tree: never run `git stash`, `git checkout`, `git restore` or
  `git reset`. Delete with `git rm` or `rm`. Kill processes by PID only.
