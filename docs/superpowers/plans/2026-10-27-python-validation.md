# Python Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `slopmeter --strict` scores a tree, the three pinned Python repos have no gate findings, and one pinned sloppy tree gates on F01–F06, F08, and F11 and warns on F07.

**Architecture:** `cli::run` parses a check or `explain`. A check calls `adapters::parse` once per selected file, measures with that language's allow-list plus the profile's extra names, joins LCOV, evaluates that language's profile, and partitions. Python's built-in gates move to the highest value a clean repo still passes. Rust and C++ keep the stand-ins. The three upstream trees are not vendored and the run is not a `cargo test`.

**Tech Stack:** Rust stable, edition 2021. No new dependencies. The corpus clones are local and discarded. radon is not involved.

## Global Constraints

- A fresh checkout has an empty diff, so the validation run is `--strict`. An empty diff gates nothing.
- Clean repos: `psf/requests`, `encode/httpx`, `pallets/click`, each at the commit recorded in `corpus/python/pins.txt`. The default branch is not the corpus.
- Thresholds may loosen until those three trees have no gate finding. A gate may not be switched off. Warns may remain, including F07. F03's warn stays below its gate.
- The sloppy tree is `corpus/python/sloppy`, written from `corpus/python/seed.txt`. The seed names no rule id and no threshold. The tree is kept only once F01–F06, F08, and F11 gate and F07 warns.
- If the only way to trip a rule is to fail a clean repo, the gate stays at the clean-repo value and the rule comes back for a decision.
- The nine-repo run is not added to CI. `cargo test` scores the sloppy tree from the files already in the repo. It does not clone GitHub.
- `explain` still prints one page and does not read a repo. When Python's number differs from the stand-in, that page lists both. The line still begins `Shipped stand-in:`.
- A wrong argument, a bad config, a missing adapter, a bad coverage report, and a diff that cannot be computed exit 2 with the reason on stderr and nothing on stdout.
- Rust and C++ files are selected and then refused with `no Rust adapter for <path>` or `no C++ adapter for <path>` until those adapters exist. The pipeline does not match on language itself. `adapters::parse` and `adapters::allow` are the arms.

---

### Task 1: Score a tree from the command

**Files:**
- Create: `src/check.rs`
- Modify: `src/adapters/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Test: `tests/check_command.rs`

**Interfaces:**
- Consumes: `inputs::select`, `inputs::load_config`, `inputs::cover`, `inputs::coverage_path`, `adapters::python::parse`, `analyzers::measure`, `rules::evaluate`, `scope::partition`, `reporters::render`
- Produces: `cli::run(cwd: &Path, args: &[String]) -> Rendered`. `slopmeter [flags] [path]` scores `path` or the working directory. Flags are `--strict`, `--json`, `--tests`, `--coverage <path>`, `--coverage=<path>`, `--changed <ref>`, and `--changed=<ref>`.

- [x] **Step 1: Write the failing test**

`tests/check_command.rs` runs the binary against a temp tree. A function with one more ordinary parameter than `Config::builtin().python.f04_gate` makes `--strict` exit 1, names `F04`, and leaves stderr empty. The same tree with no such function exits 0.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test check_command -- --nocapture`
Expected: FAIL because the binary exits 0 without scoring.

- [x] **Step 3: Wire the check**

`adapters::parse` and `adapters::allow` dispatch on `Language`. `check::score` loads config, applies `--coverage`, selects files, drops paths the config does not retain, reads bytes, parses, measures, covers, evaluates per file with that file's profile, and partitions. `main` prints the rendered check. `explain` is unchanged.

- [x] **Step 4: Add the rest of the command tests and run them**

Cover `--json`, an unknown argument, two paths, a missing value, a directory that is not a git repo, a clean git repo whose over-gate function is inherited unless `--strict`, a `.rs` file, an exclude entry, and `--coverage` that leaves one function at coverage 0.

Run: `cargo test --test check_command --test explain_command`
Expected: PASS.

---

### Task 2: Loosen the Python gates from the clean repos

**Files:**
- Modify: `src/inputs/config.rs` only if a Python gate moves
- Modify: `docs/explain/*.txt` for each gate that moves
- Modify: `tests/rules/*` and their goldens if a moved gate stops a fixture from tripping only its rule
- Create: `corpus/python/pins.txt`

**Interfaces:**
- Consumes: the binary from Task 1
- Produces: `Config::builtin().python` gates at `max(stand-in, highest value in the three trees)`. Rust and C++ stay on the stand-in constants.

- [x] **Step 1: Clone the three repos at depth 1 and record `HEAD`**

The clones live outside the work tree. `corpus/python/pins.txt` records `owner/repo <sha>`.

- [x] **Step 2: Run `--strict --json` on each clone**

A gate finding moves that rule's Python gate up to the measured value, which then passes. Re-run until no gate finding remains. Do not lower a gate. Do not add the clones or this command to CI.

- [x] **Step 3: Update the shipped Python profile, the explain pages, and the rule fixtures**

A value equal to the new gate passes. The rule-file suite still has one finding per file, and that finding is the file's rule.

- [x] **Step 4: Run the suite**

Run: `cargo test --test rule_files --test check_command --test explain_command`
Expected: PASS.

---

### Task 3: Pin the sloppy tree

**Files:**
- Create: `corpus/python/seed.txt`
- Create: `corpus/python/sloppy/`
- Test: `tests/python_corpus.rs`

**Interfaces:**
- Consumes: `cli::run` and the Python profile from Task 2
- Produces: a strict check of `corpus/python/sloppy` whose gating findings are exactly the set F01, F02, F03, F04, F05, F06, F08, F11, plus an F07 warn. Exit code 1.

- [x] **Step 1: Write the failing test**

The test points the binary at `corpus/python/sloppy` with `--strict` and the tree's coverage report. It asserts those rules and no others.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --test python_corpus`
Expected: FAIL because the tree is not there.

- [x] **Step 3: Write the seed, then the tree**

The seed asks for one small program with a function that decides too much, one that only forwards, a chain through several objects, too many parameters, a body long enough to trip the shipped length gate, a long run of statements, and a complex function the tests never reach. Add tests that can emit LCOV, and commit the report the check reads. If a required rule is missing, revise the seed without naming a rule or a threshold and write the tree again.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS. CI is still fmt, clippy, and `cargo test`.

- [x] **Step 5: Commit the code**

```bash
git add src tests corpus docs/explain
git commit -m "Score a tree and pin the Python validation run."
```
