# Root Rust member chains on the self node

Type: task
Status: resolved
Blocked by: 

## Question

`peel` at `src/adapters/rust.rs:1136` roots a chain only on an `identifier`; tree-sitter-rust emits `self` as a `self` node, so every `self.a.b.c` chain is bucketed as other (limit 11) and the receiver gate (limit 2) has never fired: `self.b.c.d.e.f` passes (review finding 4, verified). Add the `self` kind, assert `rooted` in the adapter test at `:1722`, and note that the Rust F08 other-hop limit of 11 was calibrated on mis-bucketed chains and is re-set by the recalibration ticket.

## Answer

`peel` treats a `self` node like an identifier, so `self.get().copy()` on `Repo::kept` is a rooted chain and `a.b[0].c()` on the same receiver is not. F08 then uses the receiver hop limit of 2 for those chains. `RUST_F08_OTHER_HOPS` stays 11: that number was measured while `self` chains were counted as other hops, and [Recalibrate the gates by percentile on a production-only set](46-recalibrate-gates.md) replaces it. The code-level plan is [Rust self chain root](../../../docs/superpowers/plans/2026-10-27-rust-self-chain-root.md).
