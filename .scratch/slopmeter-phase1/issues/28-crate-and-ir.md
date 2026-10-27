# Build the crate and the IR types

Type: task
Status: resolved
Blocked by: 26

## Question

Implement session 1 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md): the `slopmeter` crate, the in-memory IR types, the hand-built function test, and the fmt/clippy/test workflow.

## Answer

Done on `main`. `eef86a9` adds the IR types. `74f99f3` adds the workflow. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass. The hand-built `get` function test is the one test.
