# What slopmeter scores inside a Rust macro invocation

Type: grilling
Status: resolved
Blocked by: 

## Question

Everything inside a macro invocation is invisible to F01, F03, F05 and F08 because `walk_macro` at `src/adapters/rust.rs:863` counts only `assert*` names and function discovery at `:122` skips `macro_invocation`; `matches!(a, 1|2|…|12 if a>0)` and `dbg!(if … {…})` cost nothing (review finding 5). tree-sitter exposes the body only as a `token_tree`. Which macros does the adapter descend into, and with what rule: a fixed list of std macros (`matches!`, `dbg!`, `format!`, `println!`, `vec!`, `assert*`) parsed as expressions, a token-level heuristic for any macro (count `|` alternatives and `if` guards, `&&`/`||` runs, and field chains in any token tree), or a per-function note that a macro body was skipped? What about a `macro_rules!` defined in the repo whose body has branches? Resolve with a fixture per macro and the ADR 0003/0020 rules.

## Answer

Resolved 2026-10-27 with Christopher. Why a Rust macro is scored from its tokens: [ADR 0024](../../../docs/adr/0024-rust-macro-tokens.md). The glossary term is **Function** in `CONTEXT.md`. The build is [Score Rust macro token trees](67-rust-macro-tokens.md).

An invocation inside a function is scored on that function, from its token tree. Every invocation is scored, including one whose macro is defined in the repo. A module-level invocation is not a function and is not scored. A `macro_rules!` right-hand side is scored where it is written: on the enclosing function when the definition sits inside one, and on the macro itself when the definition sits outside any function. The left-hand pattern adds nothing, and another arm adds nothing by itself. A call does not replay the template. A macro from another crate is invisible because its text is not in the file.

`if`, `while`, `for`, and `loop` are one decision and one structural cognitive increment. The `{…}` group that follows is one nesting level deeper. `else` is one hybrid cognitive increment, and its `{…}` group is one level deeper. `else if` is one decision on top of that `else`, and that `if` is not structural. `match` is one structural increment and not a decision. An `if` before `=>` is a guard: one decision, and no cognitive increment. Each `&&` and `||` is one decision, and a cognitive increment when the operator changes. A parenthesis starts a new run. `assert!`, `assert_eq!`, `assert_ne!`, and the `debug_assert*!` names are one decision and no cognitive increment. A run of `.` names is a member chain. `self` roots it. A `(` right after a name marks that hop as a call. A comma, a semicolon, a brace, `::`, `..`, or a keyword ends the run. The allow-list and a trailing `build` still drop hops by name. An `fn` with a `{…}` body is its own function, named by the identifier after `fn`. Its tokens are scored there, starting at depth 0.

`|` adds nothing. A `matches!` of only or-patterns stays free. A closure inside a macro does not raise depth. Logical lines inside a macro stay on [Count Rust expression-position bodies and nesting per the ADRs](56-rust-lloc-and-nesting.md).
