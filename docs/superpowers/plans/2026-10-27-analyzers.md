# Analyzers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `analyzers::measure` turns one classified file into cyclomatic complexity, cognitive complexity, nesting, parameter count, chain depth, and the forward ratio.

**Architecture:** `src/analyzers.rs` is a pure function over the IR. It copies the symbol and derives the numbers. It does not read source text, and it does not match on language. The caller passes the allow-list: the adapter's defaults plus any names the project added. Thresholds stay in the rule engine.

**Tech Stack:** Rust stable, edition 2021. No new dependencies.

## Global Constraints

- Cyclomatic complexity is 1 plus the decision count. Nesting is the deepest decision depth, or 0.
- Cognitive complexity sums the increments: 1 plus depth for structural, and 1 for hybrid and fundamental.
- Parameter count excludes the receiver. That count is the denominator of the forward ratio.
- The forward ratio is the forwarded names over that parameter count. No forwarding call means zero names forwarded. The line is the forwarding call's line. The 80% line is not applied here.
- Chain depth is the number of hops that remain after two drops. An allow-listed member name adds no hop. A trailing run of calls that ends in `build` adds none either; hops before that run still count. `build` matches that exact name, and only when the last hop is a call.
- A rooted chain and any other chain are kept apart, because their limits differ and this stage does not know the limits. Each side keeps the greatest depth. An equal depth keeps the earlier line. Depth 0 is absent.
- Measurements follow the file's function order. The symbol is the path, name, parameter types, qualifier, and body span.

---

### Task 1: Measure one file

**Files:**
- Create: `src/analyzers.rs`
- Modify: `src/lib.rs`
- Test: `src/analyzers.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::{File, Function, Body, Chain, Cognitive, CognitiveKind, Forward, Span}`
- Produces: `slopmeter::analyzers::{measure, Measurement, ChainDepth, Forwarding}`

```rust
pub fn measure(file: &File, allow: &[&str]) -> Vec<Measurement>
```

`Measurement` carries `path`, `name`, `param_types`, `qualifier`, `span`, `cyclomatic`, `cognitive`, `nesting`, `params`, `receiver_chain`, `other_chain`, and `forward`. `ChainDepth` is `{line, depth}`. `Forwarding` is `{line, forwarded}`.

- [x] **Step 1: Write the failing test**

Hand-built functions. `measure` is not defined, so the tests fail to compile.

- [x] **Step 2: Derive the six measurements**

Implemented in `src/analyzers.rs`.

- [x] **Step 3: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 4: Commit**

```bash
git add src/analyzers.rs src/lib.rs
git commit -m "Derive the measurements from a classified file."
```
