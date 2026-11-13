# Parameters (`params`)

The number of arguments in a function signature. Back to [README.md](README.md).

## Why it matters

Long argument lists are easy to call with values in the wrong order, and they hint that the
function does several jobs or that a concept lacks a name. Limit in this repo: 6 for `tag:domain`.

## Counting rules

Every parameter in the signature counts, except `self`, `&self` and `&mut self` (the clippy
convention).

| Signature | Score |
| --- | --- |
| `fn f()` | 0 |
| `fn m(&self, a: i32, b: i32)` | 2 |
| `fn o(&mut self)` | 0 |
| `fn m(self: &Self, a: i32)` | 2 (typed `self` counts as a parameter) |
| `fn f(a, b, c, d, e, f, g)` (7 parameters) | 7 |

Closure parameters and generic parameters are not counted.

## Examples

```rust
fn six_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
impl S {
    fn m(&self, a: i32, b: i32) {}
    fn o(&mut self) {}
}
```

`six_params` scores 6, `S::m` scores 2 and `S::o` scores 0 (verified). The `self: &Self` form in
the table is also verified (2).

## How to reduce it

Group values that travel together into a type with a domain meaning. Do not make a bag of
unrelated values.

```rust
// before: 7
fn draw(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {}
// after: 2
fn draw(rect: Rect, color: Color) {}
```

Both scores verified.

## Known limits

- Types are not looked at: a `struct` argument counts as 1 however many fields it has.
- A typed `self` (`self: &Self`) is counted, unlike the plain forms.
- A function in a trait with no body is not measured.
