# Cognitive complexity increments

Type: grilling
Status: resolved
Blocked by: 06, 07

## Question

F03 is not cyclomatic complexity. [SonarSource cognitive complexity rules mapped to Python, Rust, C++](03-cognitive-complexity.md) says to implement Sonar white paper v1.7 Appendix B, and leaves these disagreements open: recursion (unimplemented in every analyzer), lambda and nested-function nesting (sonar-python does not raise nesting for a lambda; Rust and C++ do), and `match`/`switch` (sonar-python adds nothing; Rust and C++ add 1 and raise nesting). Cyclomatic complexity already folds a lambda's branches into the enclosing function and counts each specified match label. Which of those Sonar increments does F03 use, where does it follow the cyclomatic rules instead, and which labelled jumps count? Threshold numbers stay in the calibration fog. Clippy's `cognitive_complexity` lint is a different metric and is not the source.

## Answer

Resolved 2026-10-27 with Christopher. Why F03 follows the white paper, and which three increments it drops: [ADR 0020](../../../docs/adr/0020-cognitive-complexity-increments.md). The glossary term is **Cognitive complexity** in `CONTEXT.md`.

One table applies in Python, Rust, and C++. Depth starts at 0. An `if`, a ternary, a `match` or `switch`, a loop, and a `catch` are worth 1 plus their depth, and they raise depth for what is inside them. `elif` and `else`, including the `else` on a `for`, a `while`, or a `try`, are worth 1. They raise depth for their body and do not add their own depth. A run of like boolean operators is worth 1, and a new run starts when the operator changes. A labelled jump is worth 1. Nesting multiplies neither of those.

`match` and `switch` are one increment. Labels, a guard, `default`, and `_` add nothing. A lambda adds nothing and raises depth for its body, which stays on the enclosing function. A nested function starts at depth 0, and its increments stay on it.

These add nothing: recursion, a plain `break`, `continue`, or `return`, Rust `?`, a comprehension's `for` and `if`, `try`, `finally`, `with`, and `#if` / `#ifdef` / `#elif`. Both arms of a preprocessor condition are scored, and a literal `#if 0` still drops that arm.

The function carries each increment as line, kind, and depth. The measurement is their sum. The finding is that total, at the `def` or `fn` line. The threshold stays with calibration.
