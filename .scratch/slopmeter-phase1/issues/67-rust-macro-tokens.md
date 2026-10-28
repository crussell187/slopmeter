# Score Rust macro token trees

Type: task
Status: open
Blocked by: 51

## Question

Implement [A Rust macro is scored from its tokens](../../../docs/adr/0024-rust-macro-tokens.md). An invocation inside a function is scored on that function from its token tree. `|` adds nothing. A module-level invocation is not a function. A `macro_rules!` right-hand side is scored where it is written, and an `fn` with a body inside a macro is its own function. Logical lines inside a macro do not change. The code-level plan is [Rust macro tokens](../../../docs/superpowers/plans/2026-10-27-rust-macro-tokens.md).
