# C++ Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `adapters::cpp::parse` scores a C++ file, `--strict` on the three pinned C++ repos has no gate findings, and one pinned sloppy tree gates on F01–F06, F08, and F11 and warns on F07.

**Architecture:** `src/adapters/cpp.rs` parses with `tree-sitter` 0.27.1 and `tree-sitter-cpp` at commit `c009222808634c1014f82438d4883753516a2c24`. `adapters::parse` and `adapters::allow` gain the C++ arm. Analyzers, the rule engine, and the reporters stay language-free. C++'s built-in gates move to the highest value a clean repo still passes. Python and Rust stay on their calibrated gates. The three upstream trees are not vendored and the run is not a `cargo test`.

**Tech Stack:** Rust stable, edition 2021, `tree-sitter` 0.27.1, `tree-sitter-cpp` git rev `c009222808634c1014f82438d4883753516a2c24`.

## Global Constraints

- A function is a named body. A prototype, `= default`, `= delete`, and a pure virtual are not. A template definition is one function. An explicit specialization is its own function. A lambda inside a function is not its own function. A lambda bound to a simple name outside any function is. A lambda with neither a name nor an enclosing function is not.
- The qualified name joins namespaces, classes, and enclosing functions with `::`. An out-of-line declarator keeps the qualifiers written on it. Parameter types and the cv/ref qualifier sit beside the name. The return type and the parameter names do not.
- The body span starts at the definition. The touch span starts at the first outer attribute. `//` waiver lines directly above that anchor join the touch span. The adapter records them and does not apply a gate.
- The receiver is an explicit object parameter when one is written, and `this` for any other member function. A lambda's decisions, cognitive increments, and statements stay on the enclosing function and the lambda raises nesting. A nested function's body stays on that function.
- `ALLOW` is `begin`, `end`, `cbegin`, `cend`, `rbegin`, `rend`, `crbegin`, `crend`, `size`, `empty`, `at`, `front`, `back`, `data`, `find`, `c_str`, `value`, `value_or`, `has_value`. `.` and `->` are hops. `::` is not. A call, an index, and parentheses do not break a chain. `move` and `forward`, qualified or not, forward a parameter.
- Both arms of `#if`, `#ifdef`, `#ifndef`, `#elif`, and `#else` count. A condition that is the token `0`, parentheses allowed, drops that arm. `#if 0 && FLAG` counts. The condition itself adds nothing. A `#define` body is not a function. Macro bodies stay unscored. `assert` adds one cyclomatic decision and no cognitive increment. `goto` adds one cognitive increment and no decision.
- A fresh checkout has an empty diff, so the validation run is `--strict`.
- Clean repos: `fmtlib/fmt`, `google/re2`, `CLIUtils/CLI11`, each at the commit recorded in `corpus/cpp/pins.txt`.
- Thresholds may loosen until those three trees have no gate finding. A gate may not be switched off. F03's warn stays below its gate. F02 stays 8 when no coverage report is present.
- The sloppy tree is `corpus/cpp/sloppy`, written from `corpus/cpp/seed.txt`. The seed names no rule id and no threshold. The tree is kept only once F01–F06, F08, and F11 gate and F07 warns.
- The nine-repo run is not added to CI. `cargo test` scores the sloppy tree from the files already in the repo. It does not clone GitHub.
- When C++'s number differs from the stand-in, the explain page lists it beside the Python and Rust numbers. The line still begins `Shipped stand-in:`.

---

### Task 1: Parse C++ functions

**Files:**
- Modify: `Cargo.toml` (`tree-sitter-cpp` git rev `c009222808634c1014f82438d4883753516a2c24`)
- Modify: `src/adapters/mod.rs`
- Create: `src/adapters/cpp.rs`
- Modify: `tests/check_command.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::{File, Function, Language, Body, WaiverLine}`
- Produces: `slopmeter::adapters::cpp::{parse, ALLOW}`. `adapters::parse(Language::Cpp, path, source)` returns that file.

- [x] **Step 1: Write the failing test**

`parse` is not defined, so the qualified-method test fails to compile. A second test, once the function exists, records parameter types and a `const` qualifier, a folded lambda, a namespace-level lambda bound to a name, a `//` waiver on the touch span, a dropped `#if 0` arm, and the classification counts from the ADRs.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --lib adapters::cpp -- --nocapture`
Expected: FAIL to compile.

- [x] **Step 3: Implement `parse`**

Walk `function_definition` whose body is a compound statement or a function try block. Skip a prototype, `= default`, `= delete`, and `= 0`. Publish `ALLOW`. `adapters::allow` returns it. A `.hpp` file in the check command parses.

- [x] **Step 4: Run the adapter tests and the check command**

Run: `cargo test --lib adapters::cpp --test check_command`
Expected: PASS.

---

### Task 2: Loosen the C++ gates from the clean repos

**Files:**
- Modify: `src/inputs/config.rs` only if a C++ gate moves
- Modify: `docs/explain/*.txt` and `src/explain.rs` for each gate that moves
- Create: `corpus/cpp/pins.txt`

**Interfaces:**
- Consumes: `slopmeter --strict --json`
- Produces: `Config::builtin().cpp` gates at `max(stand-in, highest value in the three trees)`. Python and Rust stay put.

- [x] **Step 1: Clone the three repos and record `HEAD`**

The clones live outside the work tree. `corpus/cpp/pins.txt` records `owner/repo <sha>`.

- [x] **Step 2: Run `--strict --json` on each clone**

A gate finding moves that rule's C++ gate up to the measured value. Re-run until no gate finding remains. Do not lower a gate. Do not add the clones or this command to CI.

- [x] **Step 3: Update the shipped C++ profile and the explain pages**

A value equal to the new gate passes.

- [x] **Step 4: Run the suite that locks the pages**

Run: `cargo test --lib explain --test explain_command --test check_command`
Expected: PASS.

---

### Task 3: Pin the sloppy tree

**Files:**
- Create: `corpus/cpp/seed.txt`
- Create: `corpus/cpp/sloppy/`
- Test: `tests/cpp_corpus.rs`

**Interfaces:**
- Consumes: `cli::run` and the C++ profile from Task 2
- Produces: a strict check of `corpus/cpp/sloppy` whose gating findings are exactly F01, F02, F03, F04, F05, F06, F08, F11, plus an F07 warn. Exit code 1.

- [x] **Step 1: Write the failing test**

The test points the binary at `corpus/cpp/sloppy` with `--strict` and the tree's coverage report. It asserts those rules and no others. `pins.txt` records the clean commits and the C++ gates.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test cpp_corpus`
Expected: FAIL because the tree is not there.

- [x] **Step 3: Write the seed, then the tree**

The seed asks for one small program with a function that decides too much, one that only forwards, a chain through several objects, too many parameters, a body long enough to trip the shipped length gate, a long run of statements, and a complex function the tests never reach. Add tests that can emit LCOV, and commit the report the check reads.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS. CI is still fmt, clippy, and `cargo test`.
