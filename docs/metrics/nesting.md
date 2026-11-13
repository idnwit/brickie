# Nesting depth

How many branches, loops or closures sit inside each other at the deepest point of a function.
Back to [README.md](README.md).

## Why it matters

Each level keeps one more condition in the reader's head. Deep code also drifts to the right and
is hard to change safely. Limit in this repo: 4 for `tag:domain`, 6 as the ceiling.

## Counting rules

The function body is level 0. The score is the highest level reached, counting the construct
itself.

| Construct | Effect |
| --- | --- |
| `if`, `match`, `for`, `while`, `loop`, `let ... else` | the construct is at `nest + 1`; its body is one level deeper |
| `else if` | same level as the `if` it continues |
| plain `else` | its content is one level deeper than the `if` |
| closure | adds one level for its body (a closure alone scores 1) |
| macro bodies, nested `fn` | ignored |

A function with no branch scores 0.

## Examples

```rust
fn deep(rows: &[Vec<i32>]) {
    for r in rows {                    // depth 1
        if r.is_empty() { continue; }  // depth 2
        match r.len() {                // depth 2
            1 => x(),
            _ => y(),
        }
    }
}
```

Score 2 (verified).

```rust
fn count_small(rows: &[Vec<i32>]) -> usize {
    let mut n = 0;
    for row in rows {                  // 1
        for x in row {                 // 2
            if *x > 0 && *x < 10 { n += 1; }  // 3
        }
    }
    n
}
```

Score 3 (verified).

```rust
fn else_nest(a: bool, b: bool) {
    if a { x(); }                      // 1
    else {
        if b { y(); }                  // 2: inside the else
    }
}

fn closure_nest(v: &[i32]) {
    v.iter().for_each(|x| {            // closure: 1
        if *x > 0 { p(x); }            // 2
    });
}
```

Both score 2 (verified). Writing `else if b` instead of `else { if b }` would score 1.

## How to reduce it

Invert conditions and return early.

```rust
// before: nesting 3
fn flag(a: bool, b: bool, c: bool) {
    if a { if b { if c { run(); } } }
}
// after: nesting 1
fn flag(a: bool, b: bool, c: bool) {
    if !(a && b && c) { return; }
    run();
}
```

Both scores verified. `let ... else` also flattens `if let` pyramids.

## Known limits

- Branches inside macro bodies (`println!`, `vec![]`, `view! {}`) are not seen.
- A closure costs a level even when it holds no branch.
- Only branching constructs count. Plain blocks, `unsafe` and `async` blocks do not.
