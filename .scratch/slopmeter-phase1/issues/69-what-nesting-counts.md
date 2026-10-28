# Whether nesting is the deepest decision or the deepest structure

Type: grilling
Status: open
Blocked by:

## Question

F05 is the deepest decision. A decision outside any other is depth 1, and a function with no decisions has nesting 0 (`CONTEXT.md`, [ADR 0022](../../../docs/adr/0022-rule-ids-and-stand-in-thresholds.md)). The review asked to count structural levels only, so a boolean operator, a lambda, and a ternary operand would add none (finding 13). [Count Rust expression-position bodies and nesting per the ADRs](56-rust-lloc-and-nesting.md) left the definition in place.

Measured on the current adapters:

- `return 1 if a else 2 if b else 3` is nesting 2. A third ternary makes it 3.
- `return a and b` is nesting 1. `if a and b` is nesting 1. A boolean inside an `if` body is nesting 2, the same depth as any other decision there.
- `return 1 if a and b else 0` is nesting 2, because the ternary walks its condition one level down. An `if`'s condition does not.
- `return lambda y: y if a else 0` is nesting 2. The lambda is not a decision, and the ternary is the only one. The same shape in Rust and C++ is nesting 2.

Decide whether nesting stays the deepest decision, in which case the ternary condition and the lambda walk are bugs against that sentence, or whether it becomes the deepest structural level — an `if`, a loop, a `match` or `switch`, a ternary, a `catch` — so a boolean and a lambda add none. The gate stays with [Recalibrate the gates by percentile on a production-only set](46-recalibrate-gates.md).
