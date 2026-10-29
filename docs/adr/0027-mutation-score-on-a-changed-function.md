---
status: accepted
date: 2026-10-28
---

# A changed function passes when every valid mutant is detected

S06 passes a test that runs every line and asserts nothing. S07 is the gate for that hole. It reads a mutation report the caller supplies and does not run the mutation tool. The mutation score is the detected mutants over the detected plus the undetected ones, joined on the body span. A test failure and a timeout are detected. A mutant the tests pass, including one no test executed, is undetected. A mutant that never compiled is left out. The gate is 1. The score 1 passes, and a score below 1 fails. There is no warn. CRAP stays on line coverage.

A mutant belongs to the function whose body span contains the line where that mutant starts, in the same file. The report's function name is ignored. A mutant in no body span belongs to none. A function with no valid mutants has an absent score, and the rule stays off it. The finding weighs 10. It sits on the first undetected mutant, or on the function line when no score was produced. The cell reads `0.5 < 1`, or `absent < 1`. The principle is "Every valid mutant of a changed function is detected." The fix shape names the function, the measured score or its absence, and the gate. The anti-fix is a test that does not fail on the mutant.

A touched function fails when its score is below 1, when its file is absent from the report, or when the report lists an unfinished mutant in that file. A missing report fails the default run once a function is touched, and the note names the path. An empty diff still passes. A path that is set and cannot be parsed exits 2. A path that is unset, or set and missing on disk, is the missing-report case. A waiver line that names S07 fails the run.

`--strict` does not fail on S07. A missing report stays a note and adds no row. Every shown row stays inherited, so a diff run and a `--strict` run score the same tree when a score was produced. A run outside git uses that same placement.

Python reads mutmut's `mutants/<file>.meta` store. Killed and timeout are detected, survived and no-tests are undetected, and a type-check rejection is left out. `export-cicd-stats` is ignored. Rust reads cargo-mutants' `mutants.out/outcomes.json`: caught and timeout are detected, missed is undetected, and unviable is left out. C++ reads mull's mutation-testing-elements JSON. Killed and timeout are detected, survived and no-coverage are undetected, and a compile error or a runtime error is left out. Slopmeter does not read `compile_commands.json`. A skipped, suspicious, interrupted, pending, ignored, or not-checked mutant is unfinished.

## Considered options

- **Run cargo-mutants, mutmut, or mull inside the binary.** Rejected. Coverage already reads a report the caller supplies. Producing mutants stays outside the tool.
- **Replace the uncovered fraction in CRAP.** Rejected. A function can be fully line-covered and still miss a mutant. One number would answer both questions, and S06 already gates the lines.
- **Warn at 0.8 and gate at 0.6.** Rejected. A bar below 1 is slack, and the agent stops once the score clears it. An equivalent mutant can make 1 unsatisfiable. The tool cannot tell that mutant from a missing assertion, so it does not budget for it.
- **Score 1 when a function has no valid mutants.** Rejected. That says the tests caught every mutant. Scoring 0 says the mutants were seen and missed.
- **Join on the function name in the report.** Rejected. cargo-mutants names are missing outside a function, and mull has no qualified name this tool uses.
- **Let a comment waive S07.** Rejected. The agent writes the comment and would skip the gate.
- **Leave C++ out of this effort.** Rejected. Mull's JSON already has a file and a start line. The compilation database stays mull's input.
- **Read `export-cicd-stats`.** Rejected. It is one aggregate and has no per-mutant line.
- **Drop an unfinished mutant from the score.** Rejected. A run could stop once the easy mutants were killed.
- **One mutation path, or a default path per language.** Rejected. The three reports are different files, and a stale directory must not count as a measurement nobody asked to read.
- **Fail `--strict` at 1.** Rejected. The clean repos are not fully mutated, and that run would fail on the first missed mutant.

## Consequences

- The explain page is `slopmeter explain S07`. F09 stays the unknown-rule id.
- [The config file records departures from the built-in profiles](0018-config-records-departures.md) gains a `[mutation]` table with `python`, `rust`, and `cpp`. `--mutation-python`, `--mutation-rust`, and `--mutation-cpp` replace that language's key for one run, under the same path rules as `--coverage`. There is no default.
- [A finding's weight is fixed for its rule and level](0025-finding-weight-is-fixed.md) includes S07 at 10. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md) does not excuse this one. [Coverage is a line fraction joined by file and range](0011-coverage-by-line-range.md) still computes CRAP.
- The readers are separate build slices. A mutant whose start line cannot be recovered is unfinished.
