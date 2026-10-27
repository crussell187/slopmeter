# Count Rust expression-position bodies and nesting per the ADRs

Type: task
Status: open
Blocked by: 

## Question

Rust F11 is inflated two to three times because `walk_if` and `walk_match` at `src/adapters/rust.rs:650` add a statement even when the `if` or `match` is a sub-expression, and a closure body adds one more at `:806`; ADR 0021 says a statement is one and a block adds nothing (review finding 12). F05 nesting in all three languages counts boolean operators, lambdas and ternary operands as levels (`1 if a else 2 if b else 3` is 3) because decisions carry `nesting+1` at `python.rs:990` and `rust.rs:906` and `analyzers.rs:77` takes the max (finding 13). Count `if`, `match` and closure bodies only in statement position, and make nesting count structural levels only. If F05's definition turns out to be a design choice rather than a bug, bring it back as a decision.
