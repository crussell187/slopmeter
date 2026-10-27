# Reporters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `reporters::render` prints one finished check, either the text table or the JSON report, and returns exit 0 or 1. A run with no check returns exit 2.

**Architecture:** The reporter receives a report whose findings are already split into touched and inherited. It sorts each list, computes the verdict and `slop_score`, and renders one of the two forms. It does not read source, parse files, or decide which functions were touched. The binary stays empty until a later session wires the pipeline.

**Tech Stack:** Rust stable, edition 2021. `serde_json` with `preserve_order` for the document.

## Global Constraints

- The verdict fails when `findings` contains a gate whose waiver does not hold. An inherited gate does not fail. A warn does not fail. Exit 0 on pass and 1 on fail. `--json` changes the stdout bytes and not the exit code.
- Each list is ordered gate before warn, then higher weight, then a larger fraction `(measured - threshold) / threshold`. Ties break by path, then by line. A missing measurement sorts after any real fraction.
- `slop_score` is the sum of both lists' weights divided by `kloc`. No lines leaves the score null. The score does not change the verdict.
- A text row is the rule, `gate` or `warn`, the function name, `file:line`, and the measured value against its threshold. Columns share a width. Parameter types stay on the JSON symbol. A waiver row adds the excused number and the reason, or the rejection sentence.
- Untouched rows sit under `inherited`. An empty list prints no rows. An empty inherited list omits the heading. A note sits above the last line. With no score, the last line is the verdict alone.
- JSON keys follow the sample: `verdict`, `scope`, `slop_score`, `kloc`, `tool_version`, `findings`, `inherited`, `notes`. A waiver object is present only when a line is attached. A run-level note has no `path`.
- `reporters::no_check` writes the reason to stderr, prints nothing, and exits 2.

---

### Task 1: Render the sample check

**Files:**
- Create: `src/reporters.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `prototype/text-output/table.txt`
- Test: `src/reporters.rs`

**Interfaces:**
- Consumes: `rules::{Finding, Level, Waiver}`
- Produces:

```rust
pub fn render(report: &Report, json: bool) -> Rendered
pub fn no_check(reason: &str) -> Rendered
```

`Report` carries `scope`, `tool_version`, `kloc`, `findings`, `inherited`, and `notes`. `Rendered` carries `stdout`, `stderr`, and `code`.

- [x] **Step 1: Write the failing tests**

The sample report renders `prototype/text-output/table.txt` and a JSON document equal to `prototype/report-schema/report.json`. Exit code is 1. JSON does not print the table heading. A held waiver passes and shows the excused number and the reason. `no_check` exits 2.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib reporters::`
Expected: FAIL to compile because `reporters` has no `render`.

- [x] **Step 3: Sort, score, and render both forms**

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/reporters.rs prototype/text-output/table.txt
git commit -m "Print the text table and the JSON report."
```
