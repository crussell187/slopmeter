# What slopmeter scores inside a Rust macro invocation

Type: grilling
Status: open
Blocked by: 

## Question

Everything inside a macro invocation is invisible to F01, F03, F05 and F08 because `walk_macro` at `src/adapters/rust.rs:863` counts only `assert*` names and function discovery at `:122` skips `macro_invocation`; `matches!(a, 1|2|…|12 if a>0)` and `dbg!(if … {…})` cost nothing (review finding 5). tree-sitter exposes the body only as a `token_tree`. Which macros does the adapter descend into, and with what rule: a fixed list of std macros (`matches!`, `dbg!`, `format!`, `println!`, `vec!`, `assert*`) parsed as expressions, a token-level heuristic for any macro (count `|` alternatives and `if` guards, `&&`/`||` runs, and field chains in any token tree), or a per-function note that a macro body was skipped? What about a `macro_rules!` defined in the repo whose body has branches? Resolve with a fixture per macro and the ADR 0003/0020 rules.
