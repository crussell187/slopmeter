# Rust Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `adapters::rust::parse` scores a Rust file, `--strict` on the three pinned Rust repos has no gate findings, and one pinned sloppy tree gates on F01–F06, F08, and F11 and warns on F07.

**Architecture:** `src/adapters/rust.rs` parses with `tree-sitter` 0.27.1 and `tree-sitter-rust` 0.24.2. `adapters::parse` and `adapters::allow` gain the Rust arm. Analyzers, the rule engine, and the reporters stay language-free. Rust's built-in gates move to the highest value a clean repo still passes. Python stays on its calibrated gates. C++ stays on the stand-ins. The three upstream trees are not vendored and the run is not a `cargo test`.

**Tech Stack:** Rust stable, edition 2021, `tree-sitter` 0.27.1, `tree-sitter-rust` 0.24.2.

## Global Constraints

- A function is a named body. A nested `fn` is its own record. A closure inside a function is not. A closure bound to a simple name outside any function is. A signature with no body is not.
- An inherent method is `Take::into_inner`. A trait-impl method is `<Take<T> as Buf>::chunks_vectored`. A trait default is `Buf::chunks_vectored`. Nested names and inline modules join with `::`.
- The body span starts at the item. The touch span starts at the first outer attribute. `//` waiver lines directly above that anchor join the touch span. The adapter records them and does not apply a gate.
- `self` is the receiver. A closure's decisions, cognitive increments, and statements stay on the enclosing function. An inner `fn` adds nothing to the parent. A Rust tail expression counts as one logical line.
- `ALLOW` is the Rust member list from [Pass-through and Demeter detection without symbol resolution](../../.scratch/slopmeter-phase1/issues/13-passthrough-demeter-exemptions.md). `::` is not a hop. `.await` is not a hop. One of `clone`, `to_owned`, or `into` with no arguments forwards a parameter.
- Macro bodies stay unscored. `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, and `debug_assert_ne` each add one cyclomatic decision and no cognitive increment.
- A fresh checkout has an empty diff, so the validation run is `--strict`.
- Clean repos: `serde-rs/serde`, `rust-lang/regex`, `tokio-rs/bytes`, each at the commit recorded in `corpus/rust/pins.txt`.
- Thresholds may loosen until those three trees have no gate finding. A gate may not be switched off. F03's warn stays below its gate. F02 stays 8 when no coverage report is present.
- The sloppy tree is `corpus/rust/sloppy`, written from `corpus/rust/seed.txt`. The seed names no rule id and no threshold. The tree is kept only once F01–F06, F08, and F11 gate and F07 warns.
- The nine-repo run is not added to CI. `cargo test` scores the sloppy tree from the files already in the repo. It does not clone GitHub.
- When Rust's number differs from the stand-in, the explain page lists it beside the Python number. The line still begins `Shipped stand-in:`.
- C++ files are still refused with `no C++ adapter for <path>`.

---

### Task 1: Parse Rust functions

**Files:**
- Modify: `Cargo.toml` (`tree-sitter-rust` = `0.24.2`)
- Modify: `src/adapters/mod.rs`
- Create: `src/adapters/rust.rs`
- Modify: `tests/check_command.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::{File, Function, Language, Body, WaiverLine}`
- Produces: `slopmeter::adapters::rust::{parse, ALLOW}`. `adapters::parse(Language::Rust, path, source)` returns that file.

- [x] **Step 1: Write the failing test**

`parse` is not defined, so the inherent-method test fails to compile. A second test, once the function exists, records a trait-impl name, a folded closure, a module-level closure bound to a const, a `//` waiver on the touch span, and the classification counts from the ADRs.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --lib adapters::rust -- --nocapture`
Expected: FAIL to compile.

- [x] **Step 3: Implement `parse`**

Walk `function_item` with a body. Skip `function_signature_item`. Publish `ALLOW`. `adapters::allow` returns it. A `.rs` file in the check command parses. A `.hpp` file still exits 2.

- [x] **Step 4: Run the adapter tests and the check command**

Run: `cargo test --lib adapters::rust --test check_command`
Expected: PASS.

---

### Task 2: Loosen the Rust gates from the clean repos

**Files:**
- Modify: `src/inputs/config.rs` only if a Rust gate moves
- Modify: `docs/explain/*.txt` and `src/explain.rs` for each gate that moves
- Create: `corpus/rust/pins.txt`

**Interfaces:**
- Consumes: `slopmeter --strict --json`
- Produces: `Config::builtin().rust` gates at `max(stand-in, highest value in the three trees)`. Python and C++ stay put.

- [x] **Step 1: Clone the three repos and record `HEAD`**

The clones live outside the work tree. `corpus/rust/pins.txt` records `owner/repo <sha>`.

- [x] **Step 2: Run `--strict --json` on each clone**

A gate finding moves that rule's Rust gate up to the measured value. Re-run until no gate finding remains. Do not lower a gate. Do not add the clones or this command to CI.

- [x] **Step 3: Update the shipped Rust profile and the explain pages**

A value equal to the new gate passes.

- [x] **Step 4: Run the suite that locks the pages**

Run: `cargo test --lib explain --test explain_command --test check_command`
Expected: PASS.

---

### Task 3: Pin the sloppy tree

**Files:**
- Create: `corpus/rust/seed.txt`
- Create: `corpus/rust/sloppy/`
- Test: `tests/rust_corpus.rs`

**Interfaces:**
- Consumes: `cli::run` and the Rust profile from Task 2
- Produces: a strict check of `corpus/rust/sloppy` whose gating findings are exactly F01, F02, F03, F04, F05, F06, F08, F11, plus an F07 warn. Exit code 1.

- [x] **Step 1: Write the failing test**

The test points the binary at `corpus/rust/sloppy` with `--strict` and the tree's coverage report. It asserts those rules and no others. `pins.txt` records the clean commits and the Rust gates.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test rust_corpus`
Expected: FAIL because the tree is not there.

- [x] **Step 3: Write the seed, then the tree**

The seed asks for one small program with a function that decides too much, one that only forwards, a chain through several objects, too many parameters, a body long enough to trip the shipped length gate, a long run of statements, and a complex function the tests never reach. Add tests that can emit LCOV, and commit the report the check reads.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS. CI is still fmt, clippy, and `cargo test`.
