---
status: accepted
date: 2026-10-27
---

# A Rust macro is scored from its tokens

Tree-sitter gives a macro invocation a name and a `token_tree`, not an expression. Scoring nothing inside that tree lets a function hide its branches in `dbg!` or `matches!` and still pass. Scoring a fixed list of standard macros leaves the same hole under a macro the list does not name. The tokens are the text the programmer wrote, so they count on the function that contains them, using the same decision list as [Cyclomatic complexity counts decision points](0003-cyclomatic-complexity-rules.md) and [Cognitive complexity follows the white paper](0020-cognitive-complexity-increments.md) wherever a token has only one meaning.

An invocation inside a function is scored on that function. Every invocation is scored, including one whose macro is defined in the repo. A module-level invocation is not a function. A `macro_rules!` right-hand side is scored where it is written: on the enclosing function when the definition sits inside one, and on the macro itself when the definition sits outside any function. The left-hand pattern adds nothing, and another arm adds nothing by itself. A call does not replay the template. A macro from another crate is invisible because its text is not in the file. An `fn` with a `{…}` body inside a macro is its own function, named by the identifier after `fn`, and its tokens are not also scored on the enclosing function.

`if`, `while`, `for`, and `loop` are one decision and one structural increment, and the `{…}` group that follows is one nesting level deeper. `else` is one hybrid increment, and its group is one level deeper. `else if` is one decision on top of that `else`, and that `if` is not structural. `match` is one structural increment and not a decision. An `if` before `=>` is a guard: one decision, and no cognitive increment. Each `&&` and `||` is one decision, and a cognitive increment when the operator changes. A parenthesis starts a new run. `assert!`, `assert_eq!`, `assert_ne!`, and the `debug_assert*!` names are one decision and no cognitive increment. A run of `.` names is a member chain. `self` roots it. A `(` right after a name marks that hop as a call. A comma, a semicolon, a brace, `::`, `..`, or a keyword ends the run. The allow-list and a trailing `build` still drop hops by name.

## Considered options

- **Leave the body unscored and report that it was skipped.** Rejected. The function still passes, so the blind spot the gate exists to close stays open.
- **Re-parse a fixed list (`matches!`, `dbg!`, `format!`, `println!`, `vec!`, `assert*`) as expressions.** Rejected. `matches!` is a pattern and a guard, a format string is not an expression, and a macro defined in the repo is the list an agent walks around.
- **Count every `|`.** Rejected. Outside a macro a pattern alternative is a decision, a bitwise `|` is not, and `|x|` is a closure parameter. The token does not say which. Counting it makes `.map(|x| x | FLAG)` several decisions inside `dbg!` and none outside it.
- **Expand a repo `macro_rules!` at every call.** Rejected. Phase 1 has no expander, and one `if` in the template would be counted once per call.
- **Score a module-level invocation as a function.** Rejected. An invocation is not a named body. An `if` in `lazy_static!` is in the same position as an `if` in a `const` body, which is already unscored.

## Consequences

- `|` inside a macro adds nothing. A `matches!` of only or-patterns stays free. A closure inside a macro does not raise depth. Both are allowed deltas against the same code outside a macro.
- Logical lines inside a macro are unchanged. [Count Rust expression-position bodies and nesting per the ADRs](../../.scratch/slopmeter-phase1/issues/56-rust-lloc-and-nesting.md) owns that count.
- [Recalibrate the gates by percentile on a production-only set](../../.scratch/slopmeter-phase1/issues/46-recalibrate-gates.md) waits on the build. Rust cyclomatic complexity, cognitive complexity, nesting, and chain depth change once these tokens count.
- The glossary term is **Function** in `CONTEXT.md`.
