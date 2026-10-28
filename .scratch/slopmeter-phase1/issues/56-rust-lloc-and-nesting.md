# Count Rust expression-position bodies and nesting per the ADRs

Type: task
Status: resolved
Blocked by: 

## Question

Rust F11 is inflated two to three times because `walk_if` and `walk_match` at `src/adapters/rust.rs:650` add a statement even when the `if` or `match` is a sub-expression, and a closure body adds one more at `:806`; ADR 0021 says a statement is one and a block adds nothing (review finding 12). F05 nesting in all three languages counts boolean operators, lambdas and ternary operands as levels (`1 if a else 2 if b else 3` is 3) because decisions carry `nesting+1` at `python.rs:990` and `rust.rs:906` and `analyzers.rs:77` takes the max (finding 13). Count `if`, `match` and closure bodies only in statement position, and make nesting count structural levels only. If F05's definition turns out to be a design choice rather than a bug, bring it back as a decision.

## Answer

An `if` or a `match` adds a logical line when it is a block child, including a tail, or the value of a match arm. The same node under a `let`, a `return`, or a call does not. Statements and tail expressions inside the arms still count. An `else if` adds none. A `while`, a `loop`, and a `for` are unchanged.

A closure's block statements still count on the enclosing function. Its expression body counts when the closure is the statement, the block's tail, or the function itself. `let pick = |y| y + 1` does not add a line for `y + 1`. `|y| y + 1` as the function tail is one line.

Nesting stays the deepest decision: a decision outside any other is depth 1 ([ADR 0022](../../../docs/adr/0022-rule-ids-and-stand-in-thresholds.md), **Nesting** in `CONTEXT.md`). Counting structural levels only would replace that definition, so it is [Whether nesting is the deepest decision or the deepest structure](69-what-nesting-counts.md). `return 1 if a else 2 if b else 3` is already nesting 2. A boolean in a ternary's condition, and a decision inside a lambda, are depth 2 even though no other decision contains them.
