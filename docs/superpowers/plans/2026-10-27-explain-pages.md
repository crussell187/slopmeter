# Explain Pages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `explain::page` returns the shipped one-page text for F01–F08 and F11, and nothing for any other rule.

**Architecture:** Each page is a plain-text file under `docs/explain/`. `explain::page` maps a rule id to that file. The command that prints a page and exits 2 on an unknown rule is a later ticket. This slice freezes the words.

**Tech Stack:** Rust stable, edition 2021. `include_str!`. No new dependencies.

## Global Constraints

- A page is the principle, the counting method, the shipped threshold, one before and one after in source text, and the anti-fix list. It stays within a page. It does not read the project file.
- The finding's anti-fix sentence is the first line of that list.
- When the languages share a number, the threshold is one line. These nine rules share their numbers.
- Threshold numbers that calibration has not set stay marked as the shipped stand-in.
- Every gate page carries one shared sentence: the comment shape, the number, and the reason. F07's page says a waiver line for it fails the run, because F07 is not a gate.
- The words match the counting rules already decided. F02 does not invent an algebraic formula. Absent coverage leaves CRAP uncomputed.

---

### Task 1: Publish the nine pages

**Files:**
- Create: `docs/explain/F01.txt` through `docs/explain/F08.txt` and `docs/explain/F11.txt`
- Create: `src/explain.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `explain::page(rule: &str) -> Option<&'static str>`. `Some` for `F01`–`F08` and `F11`. `None` otherwise.

- [x] **Step 1: Write the failing test**

`page` is missing, so the test does not compile.

- [x] **Step 2: Run the test and watch it fail**

Run: `cargo test --lib explain::tests`
Expected: FAIL to compile because `explain` is not a module.

- [x] **Step 3: Add the pages and `explain::page`**

Each gate page ends its threshold with the shared waiver sentence. F07 says a waiver line for it fails the run.

- [x] **Step 4: Run the suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS
