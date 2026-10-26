# Coverage formats: coverage.py LCOV, cargo-llvm-cov, gcov/lcov

Type: research
Status: resolved
Context: research subagent fired 2026-10-26; findings land on branch `research/coverage-formats` at `docs/research/coverage-formats.md`
Blocked by: 

## Question

For each of the three languages, what per-function coverage information can slopmeter read from the test runner's existing output? Specifically: (a) Python — `coverage lcov` output: which records appear (SF, FN, FNDA, FNF, FNH, DA, BRDA, LH, LF), whether FN records carry function names and line ranges, and how nested functions and methods are named; (b) Rust — `cargo llvm-cov --lcov` output: the same record questions, how generic instantiations and closures appear, and what names/mangling the FN records use; (c) C++ — `gcov` via `lcov --capture` (and llvm-cov for clang): FN record naming (mangled vs demangled, `--demangle-cpp`), template instantiations, inline functions in headers. For all three: are FN/FNDA records reliable enough to attribute coverage per function, or must per-function coverage be derived from DA line records intersected with the function's line range (crap4java uses JaCoCo per-method instruction counters)? Cite the LCOV format reference (geninfo man page) and each tool's docs.

## Answer

Resolved 2026-10-26 by a research subagent. Full findings: `docs/research/coverage-formats.md` on branch `research/coverage-formats` (read with `git show research/coverage-formats:docs/research/coverage-formats.md`).

- **Bottom line:** no tool's `FN`/`FNDA` records give a per-function coverage *fraction*. CRAP needs one, so per-function coverage must be derived from `DA` line records intersected with slopmeter's own function line ranges from the IR. `FNDA` is at best a corroborating entry count.
- **LCOV format (geninfo man page):** legacy `FN:<start>[,<end>],<name>` / `FNDA:<count>,<name>`; end line only on gcc 9+. From lcov 2.2 the tool writes only the new `FNL:`/`FNA:` shape (one `FNA` per alias), so the reader must parse both.
- **Python (coverage.py `lcovreport.py`, function records since 7.6.2):** `FN:start,end,dotted-name` and `FNDA:0|1`, where hit means any statement in the region executed; `DA` counts are also 0/1. Names are dotted `Class.method` / `outer.inner`; nested `def` bodies are subtracted from the parent's region; lambdas and comprehensions are not regions; since 7.17.0 duplicates get an `@<line>` suffix.
- **Rust (`cargo llvm-cov --lcov` = `llvm-cov export -format=lcov`):** `FN:<start>,<raw mangled v0 name>` with no end line; `FNDA` is a true entry count; demangling is ignored by `export` (maintainer confirms in cargo-llvm-cov issue #325; reproduced locally with llvm-cov 21). One `FN` per monomorphization at the same start line; closures are separate records (`main::{closure#0}`) starting at the closure body line inside the parent's range; the same function can appear twice across test binaries with different crate disambiguators.
- **C++ gcc/gcov via geninfo:** reads gcov JSON (`name`, `demangled_name`, `start_line`, `end_line`); emits mangled names unless `--demangle-cpp`; start/end lines on gcc 9+, but `start_line` excludes template heads and return types. lcov folds any different name at an existing start line into an alias of one function (templates, clones, lambdas); header inline functions live in the header's `SF:` section and GCC warns counts are distorted by inlining unless `-O0`.
- **C++ clang/llvm-cov:** same path as Rust. Verified empirically: two template instantiations as separate mangled `FN` records on one line, the lambda as its own `FN` record, `static` functions prefixed with `file.cpp:`, never-called functions present with `FNDA:0`, `DA` holding the combined instantiation count.
- **Recommendation:** one LCOV reader keyed on `DA` per file; coverage = covered executable lines ÷ executable lines within the IR function's range; never key on `FN` names (three incompatible naming schemes, mangled by default on two); use `FN`/`FNL` start lines and `FNDA` only as a misalignment check.
- **Caveat:** coverage.py, cargo, GCC gcov and lcov were not installed locally, so Python and gcc behaviour comes from source and docs; the clang/LLVM path was verified by experiment.
- **Implication for tickets 15 and 19:** the IR `Function` needs an accurate executable line range, and `CoverageEntry` can be per-file line hits rather than per-function records.
