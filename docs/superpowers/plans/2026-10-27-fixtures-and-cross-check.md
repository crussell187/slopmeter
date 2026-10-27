# Fixtures and CC Cross-check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One Python file per rule renders to a checked-in text table and JSON report, and a Python cyclomatic-complexity file records the delta against radon.

**Architecture:** The test reads the rule files, parses them, measures them, joins one LCOV report, evaluates the built-in profile, and partitions with `--strict`. `reporters::render` produces both goldens. The cyclomatic-complexity file is separate. Its recorded radon numbers are the cross-check. The binary still does not score a tree.

**Tech Stack:** Rust stable, edition 2021. No new dependencies. radon is measured once and recorded. The suite does not shell out to it.

## Global Constraints

- The nine files are `tests/rules/F01.py`, `F02.py`, `F03.py`, `F04.py`, `F05.py`, `F06.py`, `F07.py`, `F08.py`, and `F11.py`. Each file's findings in the combined strict report are exactly that rule.
- `tests/rules/coverage.lcov` lists `tests/rules/F02.py` with every `DA` count at 0. Every other file is noted `No coverage record matches this file.` and its CRAP stays unset. F01 stays a cyclomatic-complexity finding only, because full coverage would make CRAP equal to a complexity already over 8.
- The report scope is `strict`. Inherited is empty. `tool_version` is the crate version. `kloc` is the physical line count of the nine files divided by 1000. Weights stay 0, which is what `rules::evaluate` emits today, so `slop_score` is 0.0 and the verdict fails.
- The goldens are `tests/rules/table.txt` and `tests/rules/report.json`, byte for byte with `render`.
- `tests/cc/decisions.py` holds one function per construct. `tests/cc/delta.txt` records `name`, slopmeter, radon, and the delta. The parser must match the slopmeter column. The delta column must be slopmeter minus radon. The only non-zero delta is `or_pattern`. `with_block` is recorded at 0, because radon's code adds 0.

---

### Task 1: Lock one file per rule

**Files:**
- Create: `tests/rules/F01.py` through `F08.py` and `F11.py`
- Create: `tests/rules/coverage.lcov`
- Create: `tests/rules/table.txt`
- Create: `tests/rules/report.json`
- Create: `tests/rule_files.rs`
- Test: `tests/rule_files.rs`

**Interfaces:**
- Consumes: `adapters::python::{parse, ALLOW}`, `analyzers::measure`, `inputs::cover`, `rules::evaluate`, `scope::partition`, `reporters::render`, `Config::builtin`
- Produces: the two golden files. No new library function.

- [x] **Step 1: Write the failing test**

Each path's findings are exactly its rule. The rendered table and JSON equal the golden files.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test rule_files`
Expected: FAIL because the rule files are not there.

- [x] **Step 3: Add the nine files, the LCOV report, and the goldens**

F01 is eight flat `if`s, cyclomatic complexity 9. F02 is two `if`s whose CRAP at coverage 0 is 12. F03 nests one `if` around six more and three `else` arms, cognitive complexity 16, cyclomatic complexity 8, nesting 2. F04 has four parameters. F05 nests three `if`s. F06 pads a two-statement body with comments until the span is 21 lines. F07 forwards its one parameter. F08 reads `response.body.city`. F11 puts 20 assignments on four lines.

- [x] **Step 4: Run the test**

Run: `cargo test --test rule_files`
Expected: PASS.

---

### Task 2: Record the radon delta

**Files:**
- Create: `tests/cc/decisions.py`
- Create: `tests/cc/delta.txt`
- Create: `tests/cc_delta.rs`
- Test: `tests/cc_delta.rs`

**Interfaces:**
- Consumes: `adapters::python::parse`
- Produces: the recorded table. No new library function.

- [x] **Step 1: Write the failing test**

The parser's cyclomatic complexity equals the slopmeter column. The delta column equals that number minus the radon column. The only non-zero delta is `or_pattern`.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test cc_delta`
Expected: FAIL because `tests/cc/delta.txt` is not there.

- [x] **Step 3: Add the functions and the measured radon numbers**

One function each for a plain body, `if`, `elif`, a boolean run, a loop, `except`, a ternary, a comprehension, a loop `else`, `finally`, `with`, and `case 1 | 2 | 3`. radon is run once with `radon cc -s`. The numbers go in `delta.txt`.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add tests/rules tests/rule_files.rs tests/cc tests/cc_delta.rs
git commit -m "Lock the rule-file report and the radon cyclomatic-complexity delta."
```
