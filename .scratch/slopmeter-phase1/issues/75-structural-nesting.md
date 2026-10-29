# Count nesting as the deepest structural level

Type: task
Status: resolved
Blocked by: 69

## Question

Implement [Nesting is the deepest structural level](../../../docs/adr/0030-nesting-is-structural.md) in the Python, Rust, and C++ adapters. Nesting is the deepest of those levels, or 0. The finding stays on that line, and on the earlier line when two share the depth. The stand-in gate stays 2 until [Recalibrate the gates by percentile on a production-only set](46-recalibrate-gates.md).

Rewrite the Counting section of `docs/explain/F05.txt`, and any sentence on that page that calls nesting a decision, to this definition. Leave the threshold numbers for the explain-pages ticket.

These readings hold in all three languages:

- `return a and b` is 0. `if a and b` is 1. `return 1 if a and b else 0` is 1.
- `return lambda y: y if a else 0` is 1. A nested function's nesting stays on that function.
- `return 1 if a else 2 if b else 3` is 2. `if 1 if a else 0` is 1. `1 if (2 if a else 0) else 3` is 1. `if a: return 1 if b else 0` is 2.
- `if` / `else` is 1. `if` / `elif` / `else` is 1. `if` / `else` / `if` inside the `else` body is 2. `if` / `else if` is 1. `if` / `else { if }` is 2.
- A guard adds none: `match x { y if y > 0 => 1 }` is 1, and an `if` inside an arm is 2. A comprehension's `for` and `if` add none: `[y for y in xs if y > 0]` is 0, and `[1 if y else 0 for y in xs]` is 1.
- `try` / `if` is 1. `except` / `if` is 2. `with` / `if` is 1. `finally` / `if` is 1. `#if` / `if` is 1.
- `try` / `except A` / `except B` / `if` is 2. `try` / `except` / `try` / `except` / `if` is 3.

## Answer

Resolved 2026-10-29. Nesting is the deepest structural level in Python, Rust, and C++, or 0 when none of those forms appear. An `if`, a loop, a `match` or `switch`, a ternary, and a `catch` each add one inside the body or a ternary's value operand. A condition, an `else`, an `elif`, an `else if`, a boolean operator, a lambda, a guard, a comprehension clause, a `try`, a `finally`, a `with`, and a `#if` add none. Handlers on one `try` share one level, and so do the arms of one `match` and the cases of one `switch`. A structural form inside a lambda counts on the enclosing function. A nested function's nesting stays on that function. The finding sits on the deepest of these lines, and on the earlier line when two share that depth. The stand-in gate stays 2. `docs/explain/F05.txt` states this count and leaves the threshold numbers in place.
