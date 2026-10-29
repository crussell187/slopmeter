---
status: accepted
date: 2026-10-29
---

# Nesting is the deepest structural level

F05 is the deepest structural level in a function, not the deepest decision. Cyclomatic complexity already counts every decision, including a boolean operator, so a nesting count of those same decisions reports that a branch exists. An `if`, a loop, a `match` or `switch`, a ternary, and a `catch` each add one inside the body or a ternary's value operand. A function with none of these has nesting 0. The gate number stays with recalibration.

## Considered options

- **Keep the deepest decision, and fix the ternary condition and the lambda walk.** Rejected. A boolean would stay a level, so `return a and b` would be nesting 1, the same depth as a top-level `if`.
- **Count every indented block, including `try`, `finally`, `with`, and `#if`.** Rejected. Those indent and do not branch. A `catch` is the branch.
- **Give `else`, a guard, a comprehension clause, and each successive handler its own level.** Rejected. An `else if` chain and the handlers on one `try` are alternatives at one depth. A guard and a comprehension clause are filters, not blocks.

## Consequences

- A condition is not inside the structure. A structural form there sits at the same depth as the structure it belongs to. `if 1 if a else 0` is 1. `1 if (2 if a else 0) else 3` is 1. `1 if a else 2 if b else 3` is 2. `if a: return 1 if b else 0` is 2.
- An `else`, an `elif`, and an `else if` add none. The body sits at the same depth as the `if` body. An `if` written inside that body still adds one, so `if` / `else` / `if` is 2 and `if` / `else if` is 1. A loop's `else`, and a `try`'s `else`, sit at that construct's depth.
- The handlers on one `try` share one level, and so do the arms of one `match` and the cases of one `switch`. `try` / `except A` / `except B` / `if` is 2. `try` / `except` / `try` / `except` / `if` is 3. A guard adds none, so `match x { y if y > 0 => 1 }` is 1, and an `if` inside an arm is 2.
- These add none: a boolean operator, a lambda, a comprehension clause, a `try`, a `finally`, a `with`, and a `#if`. A structural form inside a lambda counts on the enclosing function. A nested function's nesting stays on that function. `return a and b` is 0. `if a and b` is 1. `return 1 if a and b else 0` is 1. `return lambda y: y if a else 0` is 1. `[y for y in xs if y > 0]` is 0. `[1 if y else 0 for y in xs]` is 1. An `if` in a `try`, a `finally`, a `with`, or either arm of a `#if` stays at the enclosing depth. An `if` in an `except` or `catch` is one deeper.
- The finding sits on the deepest of these lines, and on the earlier line when two share that depth.
- [Each function rule has one metric and a hard stand-in](0022-rule-ids-and-stand-in-thresholds.md) keeps the stand-in gate of 2 until [Recalibrate the gates by percentile on a production-only set](../../.scratch/slopmeter-phase1/issues/46-recalibrate-gates.md). The same table applies in Python, Rust, and C++.
