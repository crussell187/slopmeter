# Python Classification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `adapters::python::parse` fills each function's logical lines, decision points, cognitive increments, member chains, and forwarding call.

**Architecture:** `src/adapters/python.rs` walks the tree-sitter body while it builds the function. A nested `def` or `class` is one statement of the parent and its body stays on that record. A lambda is not a record; its body expression counts as one logical line on the enclosing function, and it raises depth for what is inside. Analyzers still own the sums, the allow-list, and the 80% line.

**Tech Stack:** Rust stable, edition 2021, `tree-sitter` 0.27.1, `tree-sitter-python` 0.25.0.

## Global Constraints

- `lloc` is the number of statements in the body. An empty body is 0. A docstring, a blank, a comment, and a decorator add nothing. `else`, `elif`, a match label, and a comprehension `for` or `if` add nothing. Each statement inside a body still counts. A lambda body expression counts as one logical line on the function that contains it.
- A decision has depth 1 when it is not inside another. Cyclomatic complexity is 1 plus the decision count. Nesting is the deepest decision depth, or 0.
- These add one decision: `if`, `elif`, a ternary, `for`, `while`, a comprehension `for`, a comprehension `if`, each `except`, each `and` or `or`, `assert`, a loop `else`, a `try` `else`, each specified `match` label (each `|` alternative, and not `_`), and a `case` guard.
- These add no decision and raise no decision depth: an `if`'s `else`, `try`, `finally`, `with`, `break`, `continue`, `return`, and a bitwise operator.
- Cognitive depth starts at 0. `if`, a ternary, `match`, `for`, `while`, and `except` are structural: the stored depth is the nesting, and the sum is 1 plus that depth. They raise depth for what is inside. `elif` and every `else` are hybrid: the sum is 1, and they raise depth for the body. A run of the same `and` or `or` is one fundamental increment; a changed operator starts a new run; parentheses start a new expression; `not` does not break a run. A `match` is one structural increment. Labels and guards add nothing to it. A lambda adds nothing and raises depth. A nested function starts at depth 0.
- A chain is one run of `.` hops. A call, an index, and parentheses add no hop and do not break the run. The root name is not a hop. `rooted` is true only when that root is the receiver. A bare call is one member with `call` set, and it is not rooted. Allow-list names and a trailing `build` stay in the record.
- `forward` is set only when the body is one statement, ignoring a docstring, that unwraps through `return`, `await`, and parentheses to one call, the function has an ordinary parameter, and that call's receiver and arguments contain no other call. `names` lists the ordinary parameters handed off unchanged, in parameter order. The receiver is left out. A bare forwarding call is not also a chain. A dotted chain on that call stays.

---

### Task 1: Classify the body

**Files:**
- Modify: `src/adapters/python.rs`
- Test: `src/adapters/python.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::{Body, Chain, Cognitive, CognitiveKind, Decision, Forward, Member}`
- Produces: the `body` field of each function from `slopmeter::adapters::python::parse`

- [x] **Step 1: Write the failing test**

The acceptance fixture has `choose` (`if`, `else`, `and`, a lambda) and `ship` (one forwarding call). `parse` still leaves `body` empty, so the test fails.

- [x] **Step 2: Fill lloc, decisions, cognitive increments, chains, and forward**

Implemented in `src/adapters/python.rs`.

- [x] **Step 3: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 4: Commit**

```bash
git add src/adapters/python.rs
git commit -m "Classify Python function bodies."
```
