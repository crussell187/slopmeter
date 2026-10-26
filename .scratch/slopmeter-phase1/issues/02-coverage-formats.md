# Coverage formats: coverage.py LCOV, cargo-llvm-cov, gcov/lcov

Type: research
Status: claimed
Context: research subagent fired 2026-10-26; findings land on branch `research/coverage-formats` at `docs/research/coverage-formats.md`
Blocked by: 

## Question

For each of the three languages, what per-function coverage information can slopmeter read from the test runner's existing output? Specifically: (a) Python — `coverage lcov` output: which records appear (SF, FN, FNDA, FNF, FNH, DA, BRDA, LH, LF), whether FN records carry function names and line ranges, and how nested functions and methods are named; (b) Rust — `cargo llvm-cov --lcov` output: the same record questions, how generic instantiations and closures appear, and what names/mangling the FN records use; (c) C++ — `gcov` via `lcov --capture` (and llvm-cov for clang): FN record naming (mangled vs demangled, `--demangle-cpp`), template instantiations, inline functions in headers. For all three: are FN/FNDA records reliable enough to attribute coverage per function, or must per-function coverage be derived from DA line records intersected with the function's line range (crap4java uses JaCoCo per-method instruction counters)? Cite the LCOV format reference (geninfo man page) and each tool's docs.
