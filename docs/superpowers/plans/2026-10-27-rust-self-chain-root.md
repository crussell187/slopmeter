# Rust self chain root Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust member chain that starts at `self` is rooted, so F08 uses the receiver hop limit.

**Architecture:** `peel` already returns the root name of an `identifier`. tree-sitter-rust emits the receiver keyword as a `self` node, which falls through and leaves the root empty. Treating `self` like an identifier fills that root. `record_chain` already sets `rooted` when the root equals the method's receiver, and the rule engine already gates a rooted chain at `RUST_F08_RECEIVER_HOPS` (2). The other-hop limit stays 11; recalibration replaces it.

**Tech Stack:** Rust, tree-sitter-rust, adapter tests in `src/adapters/rust.rs`.

## Global Constraints

- F08 gates a chain deeper than 2 when it starts at the receiver, and deeper than the other-hop limit otherwise.
- A listed allow-list name adds no hop. A trailing run of calls that ends in `build` adds none.
- `RUST_F08_OTHER_HOPS` stays 11 in this change. [Recalibrate the gates by percentile on a production-only set](../../../.scratch/slopmeter-phase1/issues/46-recalibrate-gates.md) replaces it.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass.

---

### Task 1: Root a chain on the self node

**Files:**
- Modify: `src/adapters/rust.rs` (`peel`, and `member_chains_keep_every_hop_and_a_call_can_forward`)
- Modify: `src/inputs/config.rs` (comment on `RUST_F08_OTHER_HOPS` only)

**Interfaces:**
- Consumes: `peel(node, source) -> Option<Peeled>` with `Peeled { root: Option<String>, hops: Vec<Hop> }`
- Produces: a `self` node peels to `root: Some("self")` and no hops. `Repo::kept`'s chain `self.get().copy()` is `rooted`.

- [x] **Step 1: Write the failing assertion**

Inside `member_chains_keep_every_hop_and_a_call_can_forward`, after the `run` assertions:

```rust
let kept = function(&file, "Repo::kept");
assert_eq!(member_names(&kept.body.chains[0]), vec!["get", "copy"]);
assert!(kept.body.chains[0].rooted);
assert!(!only.body.chains[0].rooted);
```

`only` is `a.b[0].c()` on a method whose receiver is `self`, so that chain stays unrooted. Move the `only` lookup above this assertion, or assert `!only` after `only` is bound.

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib member_chains_keep_every_hop -- --test-threads=8`

Expected: FAIL because `Repo::kept`'s chain has `rooted == false`. The hops are already `get` and `copy`.

- [x] **Step 3: Peel a self node**

In `peel`, accept the `self` kind on the same arm as `identifier`:

```rust
"identifier" | "self" => Some(Peeled {
    root: Some(node_text(node, source)),
    hops: Vec::new(),
}),
```

Above `RUST_F08_OTHER_HOPS` in `src/inputs/config.rs`:

```rust
/// Measured while `self` chains were counted as other hops. Recalibration replaces this.
pub const RUST_F08_OTHER_HOPS: u32 = 11;
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib member_chains_keep_every_hop -- --test-threads=8`

Expected: PASS. Then `cargo test --test rust_corpus -- --test-threads=8` still passes: the sloppy tree still gates F08, and the pins still say `F08 2 11`.

- [x] **Step 5: Commit**

```bash
git add src/adapters/rust.rs src/inputs/config.rs
git commit -m "Root Rust member chains that start at self."
```
