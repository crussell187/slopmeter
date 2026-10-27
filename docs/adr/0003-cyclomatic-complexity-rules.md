---
status: accepted
date: 2026-10-26
---

# Cyclomatic complexity counts decision points

The cross-check tools agree on a core and disagree at the edges, and some of their counts are accidents: radon's docs say `with` adds 1 while its code adds 0, and lizard counts a `where` clause and can count `T&&`. Copying one tool per language would freeze those accidents so the numbers could match. Slopmeter counts its own decision points. A function starts at 1. Each path choice adds 1: a condition, a loop, a handler, a boolean operator, an `assert`, and each match or switch label. A residual arm (`else`, `default`, `_`), a jump, and syntax that always runs add nothing. A lambda's branches count on the function that contains it. The cross-check still holds the line on the shared core. Everywhere else the difference is a listed delta.

## Considered options

- **Parity with radon, rust-code-analysis, or lizard.** Rejected. No one of them is right on every disagreement, so equal numbers would not mean a correct count.

## Consequences

- A Python or-pattern, a Rust `_` arm, a bare `loop`, `with`, `finally`, `#if`, and `default` are allowed deltas against at least one tool.
- F03 is a different metric. Its increments are [Cognitive complexity follows the white paper](0020-cognitive-complexity-increments.md).
- Preprocessor conditions add nothing here. How those directives parse stays on [C++ preprocessor and template handling](../../.scratch/slopmeter-phase1/issues/16-cpp-preprocessor.md).
