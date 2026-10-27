# Root Rust member chains on the self node

Type: task
Status: open
Blocked by: 

## Question

`peel` at `src/adapters/rust.rs:1136` roots a chain only on an `identifier`; tree-sitter-rust emits `self` as a `self` node, so every `self.a.b.c` chain is bucketed as other (limit 11) and the receiver gate (limit 2) has never fired: `self.b.c.d.e.f` passes (review finding 4, verified). Add the `self` kind, assert `rooted` in the adapter test at `:1722`, and note that the Rust F08 other-hop limit of 11 was calibrated on mis-bucketed chains and is re-set by the recalibration ticket.
