# Lines (`lines`)

The length of a function in source lines. Back to [README.md](README.md).

## Why it matters

A long function rarely does one thing and is hard to review in one view. Limit in this repo: 80
for `tag:domain`, 200 as the ceiling (150 as a warning in test code).

## Counting rules

`lines = last line - first line + 1`, from the first line of the function item to its closing brace. A
`pub` or `async` qualifier on that line is included.

- Blank lines, comments and every line of macro bodies count.
- A nested `fn` and its lines count in the outer function (but not its complexity).
- Attributes (`#[inline]`) and `///` doc comments above the `fn` do not count.
- A one-line function is 1.

## Examples

```rust
/// Doc line (not counted).
#[inline]                              // not counted
fn attr_lines() {                      // 1
    // comment                         // 2
                                       // 3
    let a = 1;                         // 4
}                                      // 5
```

Score 5 (verified).

```rust
fn outer() {                           // 1
    fn inner(a: bool) {                // 2
        if a {                         // 3
            x();                       // 4
        }                              // 5
    }                                  // 6
    inner(true);                       // 7
}                                      // 8
```

`outer` scores 8 and `inner` scores 5 (verified).

## How to reduce it

Split along real responsibilities: parse, decide, act. Each part gets a name and a contract.
Do not cut a function only to forward calls. Moving a block into a helper that is used once adds
a hop for the reader without reducing the work.

```rust
// before: one function reads, validates and writes (long)
fn import(path: &str) -> Result<(), E> { /* read ... validate ... write */ }
// after: each step is short and testable
fn import(path: &str) -> Result<(), E> {
    let rows = read_rows(path)?;
    let ok = validate(rows)?;
    write_rows(&ok)
}
```

This pair is a sketch, not scored.

## Known limits

- It counts physical lines, so formatting changes the score (a long chain written on one line is 1).
- Macro bodies such as `view! {..}` count fully, though their branches are invisible to the
  other metrics.
- Blank lines and comments count, so heavy commenting raises the score.
