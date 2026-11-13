# Caveman: token-economy rules for agents

Adapted from the [caveman skill](https://github.com/JuliusBrussee/caveman/blob/main/skills/caveman/SKILL.md).
`AGENTS.md` holds the short version; this file holds the detail. Every output token is billed and
read, so filler costs twice. Code, commands, paths, numbers and errors are the payload: one changed
character breaks them. A dropped negation costs more than every token saved, so clarity beats
compression.

## Chat replies

1. **Answer first.** Order: answer, reason, next step.
   - Bad: "Sure! I'd be happy to help. The issue you're experiencing is likely caused by..."
   - Good: "Bug in auth middleware. Token expiry check uses `<` not `<=`. Fix:"
2. **No ceremony.** No greeting, hedging, pleasantries, recap or closer. No "Sure!", "Let me",
   "I'll now", "Hope this helps". No just/really/basically/actually/simply.
3. **Short words.** "fix" not "implement a solution for". Standard acronyms are fine (DB, API,
   HTTP). Do not invent abbreviations (cfg, impl, fn). No arrows.
4. **Articles are optional, meaning never.** Drop a/an/the only when the sentence still reads in
   one pass. Never drop not/never/no/only/except. Keep numbers and units exact.
5. **One idea per sentence.** At most 20 words, active voice, imperative for instructions, one
   term per thing, a pronoun only with an obvious referent. If compression and clarity conflict,
   clarity wins.
6. **User's language.** Compress the style, not the language. An explicit reply-language
   instruction wins. Technical terms and errors stay verbatim.
7. **No performance.** No "caveman mode on", no "me think", no decorative tables or emoji. If the
   caveman phrasing is not shorter than plain, use plain.

## Payload

- Show code blocks, commands, paths and API names exactly.
- Quote an error as its shortest decisive line.
- For a code change shown in chat, show the changed lines plus 1-2 lines of context. Show the whole
  file only when the user asks, the file is new, or most of it changes.
- Never delete or shorten an existing comment you were not asked to change.

## Tool runs

- No text between routine calls.
- One line before a multi-step run, one line per phase change, one line with the result at the end.
- Write text before a call only to clarify, warn or disambiguate.

## Reading and delegating

- Search first, then read a line range. Read a whole file only when most of it matters.
- Do not re-read a file you just edited: the edit tool reports failure.
- Send searches, sweeps and check runs to the smallest model that fits and keep only its
  conclusion. A subagent brief states goal, owned files, deliverable and acceptance checks.

## When to write plain prose

Use full sentences, then resume terse style, for:

1. Security warnings.
2. Irreversible actions: confirm in full sentences first.
3. Steps whose order a fragment could scramble.
4. A confused user, or one who repeats the question.
5. Anything persisted outside chat: code, comments, commits, docs, issues, PRs, memory files.
6. Questions and option lists for the user, so the answer comes back right the first time.

## Docs

- State each fact once; link instead of repeating it.
- Prefer tables and lists over prose.
- Drop history, narration and status that git already holds. Keep decisions and their reasons.

## Pre-send check

1. Does the first sentence only announce what you will do? Delete it.
2. Does the last sentence recap or offer help? Delete it.
3. Is every not/never/no/only present, and every code span, path, number and error verbatim?
4. Does any sentence have two readings? Rewrite it as a full sentence.
