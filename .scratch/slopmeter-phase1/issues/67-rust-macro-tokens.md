# Score Rust macro token trees

Type: task
Status: resolved
Blocked by: 51

## Question

Implement [A Rust macro is scored from its tokens](../../../docs/adr/0024-rust-macro-tokens.md). An invocation inside a function is scored on that function from its token tree. `|` adds nothing. A module-level invocation is not a function. A `macro_rules!` right-hand side is scored where it is written, and an `fn` with a body inside a macro is its own function. Logical lines inside a macro do not change. The code-level plan is [Rust macro tokens](../../../docs/superpowers/plans/2026-10-27-rust-macro-tokens.md).

## Answer

Resolved 2026-10-28. `adapters::rust` walks every token of a macro on the function that contains it. `if`, `while`, `for`, and `loop` are one decision and one structural increment. `else` is one hybrid increment. `else if` adds the decision and not a second structural increment. `match` is structural only, and a guard `if` before `=>` is one decision and no cognitive increment. A parenthesis starts a new boolean run. `|` adds nothing. A `.` run is a member chain, rooted on `self`, and a `(` after a name marks that hop as a call. A module-level invocation is not a function. A `macro_rules!` outside any function is a function, and one inside a function scores its right-hand side on that function. An `fn` with a brace body inside a macro is its own function, and those tokens are not also scored on the enclosing function. Logical lines inside a macro are unchanged.
