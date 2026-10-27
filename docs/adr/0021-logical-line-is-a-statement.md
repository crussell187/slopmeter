---
status: accepted
date: 2026-10-27
---

# A logical line is a statement

The length rule needs a count of how much a function says, and the rulers do not agree. radon counts colons in a whole file and scores a dict literal as two. lizard counts physical lines, so a blank and a comment make a function longer. rust-code-analysis counts statement nodes per function, which is the right kind of count, and its node list still misses a Rust tail expression. A function's logical-line count is the number of statements in its body. An empty body is 0.

## Considered options

- **Physical lines.** Rejected. The signature, a blank, a comment, a docstring, a brace, and a decorator or attribute would count, and a statement broken across lines would count more than once.
- **radon's colon heuristic.** Rejected. It is not per function, and `{'a': 1}` is one statement.
- **rust-code-analysis's node list, unchanged.** Rejected. A tail expression with no semicolon would be free, and a Python nested `def` would not be a statement of its parent.

## Consequences

- A statement written across many lines is one. Two statements separated by `;` are two. An `if`, a loop, a `return`, a `try`, a `with`, a `catch`, a `let`, and a declaration each count, and so does each statement in the body.
- `else`, `elif`, a `match` or `switch` label, a comprehension's `for` and `if`, an empty `;`, and a preprocessor directive add nothing. The statements in an arm still count.
- A closure's statements count on the function that contains it. A nested function's body stays on that function. In Python, the `def` or `class` is one statement of the parent. In Rust, an inner `fn` adds nothing to the parent.
- A block adds nothing beyond what is inside it. A Rust tail expression counts as one.
- Both arms of `#if`, `#ifdef`, and `#elif` contribute their statements. A literal `#if 0` drops that arm. A macro body stays unscored.
- The field is the integer the adapter fills. The threshold stays with calibration.
