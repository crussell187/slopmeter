# Attributing coverage records to IR functions, per language

Type: grilling
Status: resolved
Blocked by: 02, 06

## Question

Given what the coverage research found about FN/FNDA/DA records from coverage.py, cargo-llvm-cov and gcov, and given the function-identity decision: how does slopmeter attribute coverage to each function? By matching FN names (needs demangling for C++, closure naming for Rust), by intersecting DA line records with the function's line range, or both with a precedence? What coverage fraction feeds CRAP (lines, instructions, branches)? What happens for a function with no coverage data (spec: N/A, warn-only) versus a file missing from the report entirely, and does the tool optionally run the test command first as crap4java does?

## Answer

Resolved 2026-10-26 with Christopher. Why coverage is a line fraction joined by file and range: [ADR 0011](../../../docs/adr/0011-coverage-by-line-range.md). The glossary terms are **Coverage** and **CRAP** in `CONTEXT.md`. The facts are on [Coverage formats: coverage.py LCOV, cargo-llvm-cov, gcov/lcov](02-coverage-formats.md).

Phase 1 reads an LCOV report it is given. It does not run `pytest`, `cargo test`, or `ctest`. The report path's place in the config file stays on [Shape of slopmeter.toml](22-config-shape.md).

A function's coverage is hit `DA` lines divided by `DA` lines in its range. The range runs from the `def` or `fn` line through the end of the body. A nested function's lines come out of the parent. A lambda or closure stays in the function that contains it. One template is one function, and a hit from any instantiation covers the line. A count above zero is a hit. `FN`, `FNDA`, `FNL`, and `FNA` are ignored. The reader still accepts a file that contains them.

An `SF` path matches a scored file when the repo-relative path is a directory-boundary suffix of it, after separators are normalized to `/` and `.` and `..` are resolved lexically. The longer scored path wins. A tie leaves that file's coverage absent and the run notes it. Several `SF` sections for one scored file merge: a line is listed if any section lists it, and it is a hit if any count is above zero.

`DA` lines all at zero are coverage 0, and CRAP runs. No listed lines means coverage is absent and CRAP is not computed. No report is one note on the run. A missing file is one note for that file. A reported file whose function has no `DA` lines is one warn on that function. None of those gate. A header function is covered only by the header's own `SF` record.
