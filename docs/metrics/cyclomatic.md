# Cyclomatic complexity

The number of independent paths through a function, which is the least number of tests that
cover every branch. Back to [README.md](README.md).

## Why it matters

Every branch is a case a test must reach. A `match` with 20 arms is easy to read but still needs
20 cases. Cyclomatic complexity counts them, where `cognitive` counts reading effort. No limit is
set in this repo.

## Counting rules

The score starts at 1 for the function itself.

| Construct | Adds |
| --- | --- |
| `if`, `else if` | 1 each |
| `for`, `while`, `loop`, `let ... else` | 1 each |
| `match` | number of arms minus 1 |
| each `&&` and each `||` | 1 per operator |
| each `?` | 1 |
| plain `else`, guards, closures, `return` | 0 |

A `match` guard (`x if x > 0 =>`) adds nothing beyond its arm.

## Examples

```rust
fn month_name(m: u8) -> &'static str {
    match m {                          // 4 arms: +3
        1 => "jan",
        2 => "feb",
        3 => "mar",
        _ => "other",
    }
}
```

Score 4 (verified): 1 + 3.

```rust
fn load(p: &str) -> Result<String, E> {
    let a = read(p)?;                  // +1
    let b = parse(&a)?;                // +1
    Ok(b)
}
```

Score 3 (verified): 1 + 2. Its `cognitive` score is 0.

```rust
fn mixed_ops(a: bool, b: bool, c: bool, d: bool) -> bool {
    a && b && c || d                   // +3 (three operators)
}
```

Score 4 (verified). Its `cognitive` score is 2.

## How to reduce it

Move a branch chain into data or a lookup, or split by responsibility.

```rust
// before: cyclomatic 6 (1 + for + 4 ifs)
fn total(items: &[Item]) -> i32 {
    let mut t = 0;
    for i in items {
        if i.kind == 1 { t += 1; }
        else if i.kind == 2 { t += 5; }
        else if i.kind == 3 { t += 7; }
        else if i.kind == 4 { t += 9; }
    }
    t
}
// after: total is 2, weight is 5
fn weight(k: u8) -> i32 {
    match k { 1 => 1, 2 => 5, 3 => 7, 4 => 9, _ => 0 }
}
fn total(items: &[Item]) -> i32 {
    let mut t = 0;
    for i in items { t += weight(i.kind); }
    t
}
```

All four scores verified. The branches still exist in `weight`, but as a flat table (cognitive 1).
A real table (`const`/`static` array or map) removes them from cyclomatic too.

## Known limits

- Macro bodies are not parsed, so a branch inside `println!(..)` is not counted.
- `?` counts even though it only forwards an error, so error-heavy code scores high.
- Arms of a `match` count the same whether trivial or not.
