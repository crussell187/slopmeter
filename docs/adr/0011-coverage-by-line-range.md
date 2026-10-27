---
status: accepted
date: 2026-10-26
---

# Coverage is a line fraction joined by file and range

CRAP needs a coverage fraction per function, and the LCOV function records do not provide one. Names differ across coverage.py, cargo-llvm-cov, and gcov, and Rust and C++ mangle them. Phase 1 reads an LCOV report it is given and does not run the tests. A function's coverage is the hit `DA` lines divided by the `DA` lines in its own range. The join is the scored file plus that range. `FN`, `FNDA`, `FNL`, and `FNA` are ignored.

## Considered options

- **Join on `FN` names, or demangle them.** Rejected. Three naming schemes, mangled by default on two, and a name is not a fraction.
- **Treat a missing report as coverage 0.** Rejected. That punishes an uninstrumented function as if the tests had reached it and run nothing. Absent lines leave CRAP uncomputed.
- **Run the test command, as crap4java does.** Rejected. Each language already has a test runner, and the C++ build system is still an open question.

## Consequences

- A nested function's lines belong to that function. A lambda or closure stays with the function that contains it. One template is one function, and any instantiation's hit covers the line.
- An `SF` path matches a scored file when the repo-relative path is a directory-boundary suffix of it. The longer path wins. A tie, a missing file, or a function with no `DA` lines leaves coverage absent and does not gate. `DA` lines all at zero are coverage 0, and CRAP runs.
- The report path's place in config stays on [Shape of slopmeter.toml](../../.scratch/slopmeter-phase1/issues/22-config-shape.md). A header function is covered only from the header's own `SF` record.
