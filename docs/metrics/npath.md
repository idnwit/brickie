# Execution paths (`paths`)

An estimate of how many different routes execution can take through a function, a simplified
NPath. Back to [README.md](README.md).

## Why it matters

Each path is a combination a test could cover. The number shows how quickly independent decisions
pile up in one function. No limit is set in this repo.

## Counting rules

`B(x)` is the paths of a branch body, never below 1. A body with no decision is 1. The score of
the function is `B(body)`. The count is capped at 1,000,000,000.

| Construct | Paths |
| --- | --- |
| sequence of statements | the sum of the decisions in it |
| `if` | `B(then) + B(else)` plus 1 per `&&`/`||` in the condition; no `else` counts as 1 |
| `else if` | the `else` body is the next `if` |
| `match` | the sum of `B(arm)`, plus 1 per `&&`/`||` in each arm pattern or guard |
| `match` with 3 or more arms that are all unguarded constants | 1 (a lookup table) |
| `for`, `while`, `loop` | `B(body) + 1` (plus 1 per `&&`/`||` in a `while` condition) |
| `let ... else` | `B(else) + 1` |
| `?` | 1 |

Constant arms are literals, identifiers, paths, field accesses, references, and calls whose
arguments are all of those.

## Examples

```rust
fn classify(n: i32) -> &'static str {
    if n < 0 { "negative" }            // B(then) 1 + else branch
    else if n == 0 { "zero" }          //   B(then) 1 + else 1
    else { "positive" }                //   (the else body is 1)
}
```

Score 3 (verified): the outer `if` is 1 + 2.

```rust
fn three_seq(a: bool, b: bool, c: bool) {
    if a { x(); }                      // 1 + 1 = 2
    if b { y(); }                      // 2
    if c { z(); }                      // 2
}
```

Score 6 (verified). Sequential decisions add. Nested `if a { if b { .. } }` scores 3 (verified):
the inner `if` (2) replaces the outer `then` body, plus 1 for the missing `else`.

```rust
fn cond_ops(a: bool, b: bool, c: bool) {
    if a && b || c { x(); }            // 2 operators + 1 + 1 = 4
}
```

Score 4 (verified).

```rust
fn shape_area(s: &Shape) -> f64 {
    match s {                          // 4 arms, one path each
        Shape::Circle(r) => 3.14 * r * r,
        Shape::Square(a) => a * a,
        Shape::Rect(a, b) => a * b,
        Shape::Empty => 0.0,
    }
}
```

Score 4 (verified). The same shape with constant arms (see below) scores 1.

## How to reduce it

Use a table for constant mappings and keep unrelated decisions in separate functions.

```rust
// before: paths 4
fn name(k: u8) -> &'static str {
    if k == 1 { "one" } else if k == 2 { "two" } else if k == 3 { "three" } else { "many" }
}
// after: paths 1
fn name(k: u8) -> &'static str {
    match k { 1 => "one", 2 => "two", 3 => "three", _ => "many" }
}
```

Both scores verified. Four independent `if`s in one function score 8 (verified); two functions
with two each score 4 each, so split only along real responsibilities.

## Known limits

- This is not textbook NPath. Sequential decisions add instead of multiply, so the score grows
  more slowly (three `if`s give 6, not 8).
- Two `?` in a row give 2 (not 3): each `?` is 1 and the body sum is taken.
- Macro bodies are not parsed. Closure bodies add their own decisions to the enclosing count.
