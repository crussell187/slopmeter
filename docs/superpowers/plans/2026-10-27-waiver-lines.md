# Waiver Lines Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `adapters::python::parse` records every waiver line in the contiguous run above a function, and extends that function's touch span to include the run.

**Architecture:** The comment text is recorded on the function as tokens. The adapter does not decide whether a line excuses a gate. A line whose first tokens are `slopmeter:` and `waive` stays in the run even when the rule, the number, or the reason is missing. Any other comment, or a blank line, ends the run. The body span is unchanged, so the lines add nothing to function length or to logical lines.

**Tech Stack:** Rust stable, edition 2021. `tree-sitter-python` 0.25.0. No new dependencies.

## Global Constraints

- Python uses `#`. The tokens are `slopmeter:`, `waive`, the rule id, and the number. The trimmed remainder is the reason.
- The run sits directly above the touch span: above a decorator when there is one, and above the signature otherwise. Those lines join the touch span.
- A comment inside a body does not waive that function. A waiver above a nested function attaches to the nested function. That line still lies in the enclosing body span.
- A number token is recorded as written, including a fraction. The adapter does not apply a gate.
- Rust and C++ record the same shape when those adapters land. This slice is Python only.

---

### Task 1: Record the run on the function

**Files:**
- Modify: `src/ir.rs`
- Modify: `src/adapters/python.rs`
- Modify: `src/analyzers.rs` (the hand-built `Function` helper gains an empty waiver list)
- Test: `src/adapters/python.rs`

**Interfaces:**
- Consumes: `adapters::python::parse(path, source) -> File`
- Produces: `ir::WaiverLine { line, rule, number, reason }` on `Function.waivers`, in source order. `rule`, `number`, and `reason` are absent when that token is missing. `touch.start` is the first waiver line when the run is non-empty.

- [x] **Step 1: Write the failing tests**

Cover a two-line run, a waiver above a decorator, a waiver between a decorator and the signature, a blank line, an ordinary comment, a comment inside the body, a nested function, an incomplete line that does not cut off the line above it, a fractional number, and a named lambda.

- [x] **Step 2: Run the tests and watch them fail**

Run: `cargo test --lib adapters::python::tests`
Expected: FAIL to compile because `Function` has no `waivers` field.

- [x] **Step 3: Record the lines**

Add `WaiverLine` and `Function.waivers`. Walk the comment siblings directly above the decorated definition, or above the function when it has no decorator. For a named lambda, walk above the assignment. Extend `touch.start` to the first recorded line. Leave `span` alone.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add src/ir.rs src/adapters/python.rs src/analyzers.rs
git commit -m "Record Python waiver lines on the function."
```
