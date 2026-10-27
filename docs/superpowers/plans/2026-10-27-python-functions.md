# Python Functions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `adapters::python::parse` turns one Python file into function records: qualified name, parameters, body span, and touch span.

**Architecture:** `src/adapters/python.rs` parses with `tree-sitter` 0.27.1 and `tree-sitter-python` 0.25.0. It walks `def` and class bodies, and a lambda only when a simple name binds it outside any function. Body classification stays empty until [Classify a Python body](../../.scratch/slopmeter-phase1/issues/32-python-classification.md).

**Tech Stack:** Rust stable, edition 2021, `tree-sitter` 0.27.1, `tree-sitter-python` 0.25.0.

## Global Constraints

- A function is a named body. A nested `def` is its own record. A lambda inside a function is not. A lambda bound to a simple name outside any function is. A bare lambda is not.
- Python names are dotted: `outer.inner`, `Response.json`.
- The body span starts at `def` or `lambda`. The touch span starts at the first decorator. A docstring does not move it.
- The first parameter of a function defined directly in a class is the receiver. `@staticmethod` has none. `@classmethod` keeps its first parameter. A nested function is not a method.
- `@overload` is not a function. `async def` is.
- A latin-1 coding cookie is decoded before parsing. `unparsed_ratio` is the fraction of lines an error node touches, and a broken file still returns a record.
- `ALLOW` is the Python member list from [Pass-through and Demeter detection without symbol resolution](../../.scratch/slopmeter-phase1/issues/13-passthrough-demeter-exemptions.md).

---

### Task 1: Parse functions

**Files:**
- Modify: `Cargo.toml` (`tree-sitter` = `0.27.1`, `tree-sitter-python` = `0.25.0`)
- Modify: `src/lib.rs`
- Create: `src/adapters/mod.rs`
- Create: `src/adapters/python.rs`
- Test: `src/adapters/python.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::{File, Function, Language, Param, Span, Body}`
- Produces: `slopmeter::adapters::python::{parse, ALLOW}`

`parse(path, source) -> File`. It does not open the path.

- [x] **Step 1: Write the failing test**

`parse` is not defined, so the nested-def test fails to compile.

- [x] **Step 2: Record defs, methods, and named lambdas**

Implemented in `src/adapters/python.rs`. The acceptance fixture records three functions, `outer`, `outer.inner`, and `bound`, because the enclosing `def` is a function too. The lambda inside `inner` is not a record.

- [x] **Step 3: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/adapters
git commit -m "Parse Python functions into the IR."
```
