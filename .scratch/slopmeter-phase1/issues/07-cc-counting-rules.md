# Cyclomatic complexity counting rules, per language

Type: grilling
Status: resolved
Blocked by: 04

## Question

Starting from crap4java's rules (base 1; +1 per if, loop, catch, ?:, switch label, &&, ||) and what the cross-check research found radon, rust-code-analysis and lizard do, which constructs increment CC in slopmeter for Python (elif, comprehension `if`/`for`, `and`/`or`, `with`, `assert`, ternary, `match` cases, except clauses), Rust (match arms, `if let`/`while let`, `?`, `&&`/`||`, let-else, closures), and C++ (switch cases incl. default, `?:`, `&&`/`||`, catch, range-for, goto, preprocessor-conditional branches)? Do we aim for parity with one existing tool per language so the validation cross-check can demand equal numbers, or document our own rules and accept a bounded delta?

## Answer

Resolved 2026-10-26 with Christopher. The rules are slopmeter's own. radon, rust-code-analysis, and lizard are a cross-check: the shared core must match, and every other difference below is an allowed delta. Why parity was rejected: [ADR 0003](../../../docs/adr/0003-cyclomatic-complexity-rules.md). The glossary term is **Cyclomatic complexity** in `CONTEXT.md`.

A function starts at 1. A lambda's branches count on the function that contains it. A nested function has its own count. There is no file-level or class-level score.

These add 1:

- `if`, `elif`, `else if`, and a ternary
- a loop: `for`, `while`, `do`/`while`, range-`for`, a bare `loop`, and a comprehension `for`
- a comprehension `if`, an `if let`, a `while let`, and `?`
- each `except` or `catch`
- each boolean `&&`, `||`, `and`, or `or`, so `a and b and c` is +2
- `assert`, a loop `else`, and a `try` `else`
- `let…else`, once
- each specified match or switch label, including each alternative in `1 | 2 | 3` and each `case` label that shares a body
- a guard `if` on an arm, in addition to the label

These add nothing:

- `else`, `default`, and `_`
- `try`, `finally`, `with`, and a `where` clause
- the lambda keyword, `goto`, `break`, `continue`, and `return`
- a preprocessor condition (`#if`, `#ifdef`, `#elif`)
- a bitwise operator, and a C++ rvalue reference (`T&&`)

Allowed deltas:

- **radon:** an or-pattern counts as one case; the docs say `with` adds 1 and the code adds 0. We add 1 per or-pattern label and nothing for `with`.
- **rust-code-analysis:** counts `_` and `with`; ignores Python `match`. We leave `_` and `with` at 0 and count each specified match label.
- **lizard:** counts arms after the first, so an exhaustive match with no `_` differs by 1; counts `finally`, `where`, and `#if`; misses a bare `loop`; can count `T&&`; counts `case _`. We do none of those.
- **crap4java:** counts `default`, because javac represents it as a case label. We leave it at 0.

F03 is not this metric. Its increments are [Cognitive complexity increments](24-cognitive-complexity-increments.md). How preprocessor directives parse stays on [C++ preprocessor and template handling](16-cpp-preprocessor.md).
