# Cognitive complexity

How hard a function is for a person to read, counting each decision more the deeper it sits.
Adapted from SonarSource. Back to [README.md](README.md).

## Why it matters

A flat chain of ten `if`s is easy to follow. Three nested loops with a condition inside are not.
Cognitive complexity charges more for nesting, so it tracks reading effort better than a plain
count of branches does. Limit in this repo: 15 for `tag:domain`, 60 as the ceiling.

## Counting rules

`nest` is the current nesting level, starting at 0 in the function body.

| Construct | Cost |
| --- | --- |
| `if`, `match` (once, not per arm), `for`, `while`, `loop`, `let ... else` | `1 + nest` |
| `else if` | `1` (no nesting increase) |
| `else` | `1` |
| a run of like `&&` / `||` operators | `1`, plus `1` per switch between `&&` and `||` |
| labeled `break` / `continue` | `1` (flat) |
| direct recursive call: `name()`, `Self::name()`, `self.name()` | `1` |
| `?`, plain `break`, `continue`, `return`, closures | `0` |

Nesting grows by 1 inside the body of an `if`, `else`, `match`, loop, `let ... else` and closure.
The condition of an `if` is not nested.

## Examples

```rust
fn classify(n: i32) -> &'static str {
    if n < 0 { "negative" }            // +1
    else if n == 0 { "zero" }          // +1 (else if)
    else { "positive" }                // +1 (else)
}
```

Score 3 (verified).

```rust
fn count_small(rows: &[Vec<i32>]) -> usize {
    let mut n = 0;
    for row in rows {                  // +1 (nest 0)
        for x in row {                 // +2 (nest 1)
            if *x > 0 && *x < 10 {     // +3 (nest 2), +1 for the && run
                n += 1;
            }
        }
    }
    n
}
```

Score 7 (verified): 1 + 2 + 3 + 1.

```rust
fn mixed_ops(a: bool, b: bool, c: bool, d: bool) -> bool {
    a && b && c || d                   // +1 for the run, +1 for the switch to ||
}

fn fact(n: u64) -> u64 {
    if n == 0 { 1 }                    // +1
    else { n * fact(n - 1) }           // +1 (else), +1 (recursion)
}
```

`mixed_ops` scores 2 and `fact` scores 3 (both verified).

## How to reduce it

Use guard clauses and early returns to remove nesting.

```rust
// before: score 5
fn price(o: Option<i32>, vip: bool) -> i32 {
    if let Some(v) = o {               // +1
        if vip { v / 2 } else { v }    // +2 (nest 1), +1 (else)
    } else { 0 }                       // +1 (else)
}
// after: score 3
fn price(o: Option<i32>, vip: bool) -> i32 {
    let Some(v) = o else { return 0 }; // +1
    if vip { v / 2 } else { v }        // +1, +1 (else)
}
```

Both scores verified.

## Known limits

- Macro bodies are not parsed, so a branch inside `println!(..)` costs nothing.
- Closures add nesting but cost nothing themselves, so `iter().filter(|x| ..)` hides no cost
  until it contains a branch.
- A labeled `break` costs a flat 1 even when deeply nested.
- Only direct recursion is detected, not mutual recursion.
- `?` is free, so long `?` chains do not raise the score (they raise `cyclomatic`).
