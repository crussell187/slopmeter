# What increments a logical line

Type: grilling
Status: resolved
Blocked by: 19

## Question

The Phase 1 IR stores one `lloc` integer per function, filled by the adapter. [How radon, rust-code-analysis and lizard count cyclomatic complexity](04-cross-check-tools.md) shows the rulers disagree: radon applies a colon heuristic at file level and over-counts a dict literal, rust-code-analysis counts statement nodes per function, and lizard reports physical length. Which lines increment `lloc` in Python, Rust, and C++, and do blanks, comments, docstrings, braces, and attributes count?

## Answer

Resolved 2026-10-27 with Christopher. Why a logical line is a statement: [ADR 0021](../../../docs/adr/0021-logical-line-is-a-statement.md). The glossary term is **Logical line** in `CONTEXT.md`.

`lloc` is the number of statements in the body. An empty body is 0. The signature, a blank, a comment, a docstring, a brace, and a decorator or attribute add nothing. A statement written across many lines is one. Two statements separated by `;` are two. An `if`, a loop, a `return`, a `try`, a `with`, a `catch`, a `let`, and a declaration each count, and so does each statement in the body.

`else`, `elif`, a `match` or `switch` label, a comprehension's `for` and `if`, an empty `;`, and a preprocessor directive add nothing. The statements in an arm still count. A Rust tail expression counts as one. A block adds nothing beyond what is inside it.

A closure's statements count on the function that contains it. A nested function's body stays on that function. In Python, the `def` or `class` is one statement of the parent. In Rust, an inner `fn` adds nothing to the parent. Both arms of `#if`, `#ifdef`, and `#elif` contribute their statements. A literal `#if 0` drops that arm. A macro body stays unscored.

The threshold stays with calibration.
