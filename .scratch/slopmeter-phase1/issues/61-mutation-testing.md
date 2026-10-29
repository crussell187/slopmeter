# Mutation testing as a slopmeter gate

Type: grilling
Status: resolved
Blocked by: 55, 60

## Question

The spec listed mutation testing as a v1 non-goal and scheduled S07 (mutation score on changed functions, CRAP′ = CC² × (1 − mutation)³ + CC) for Phase 3; the map put it out of scope and nothing in the tool reads a mutation report. Chris wants it in the chain. Decide the shape: the tool reads a report, as with coverage, and does not run the mutation tool; per language the inputs are cargo-mutants' `mutants.out/outcomes.json` (caught, missed, timeout, unviable per mutant; `--in-diff` matches diff scoping), mutmut 3's `mutants/` store or `export-cicd-stats` (no documented per-mutant JSON), and for C++ mull's mutation-testing-elements JSON or dextool, both of which need a compilation database, which the C++ macro-policy decision governs. Decide the gate level (spec: 60 gate / 80 warn), whether mutation replaces coverage in CRAP when present, how a mutant maps to a function (file and line against the body span), what a function with no mutants scores, and whether C++ gets S07 at all in this effort. The per-language report readers are build slices once this is decided.

## Comments

2026-10-27 — [How C++ function-like macros are surfaced or expanded](55-cpp-macro-policy.md) closed the compilation-database question. A C++ mutation gate may exist. Slopmeter reads the mutation report and does not read `compile_commands.json`. This ticket still decides whether this effort includes that gate.

2026-10-28 — Slopmeter reads a mutation report the caller supplies. It does not run the mutation tool.

2026-10-28 — CRAP stays on line coverage. Mutation is its own rule.

2026-10-28 — The mutation score is caught and timeout over caught, timeout, and missed. A mutant that never compiled is left out.

2026-10-28 — A function with no valid mutants has an absent mutation score, and the rule stays off that function.

2026-10-28 — A mutant belongs to the function whose body span contains the line where that mutant starts, in the same file. The report's function name is ignored. A mutant in no body span belongs to none.

2026-10-28 — The mutation rule gates at 1, with no warn, in every language. The score 1 passes. A score below 1 fails.

2026-10-28 — A missing mutation report fails the default run when a function was touched. The note names the path.

2026-10-28 — Under `--strict`, a missing mutation report stays a note and adds no finding.

2026-10-28 — When a mutation report is present, rows below 1 stay inherited under `--strict`. They count in the slop score and do not fail that verdict.

2026-10-28 — A touched function whose file is not in a present report fails the default run. A matched file with no valid mutants in the span stays off the rule.

2026-10-28 — A waiver line that names the mutation rule fails the run. The gate at 1 cannot be waived.

2026-10-28 — This effort includes the mutation rule for C++. The report is mull's mutation-testing-elements JSON. Slopmeter does not read `compile_commands.json`.

2026-10-28 — Python reads mutmut's `mutants/<file>.meta` store, one result per mutant. `export-cicd-stats` is ignored.

2026-10-28 — Rust reads cargo-mutants' `mutants.out/outcomes.json`.

2026-10-28 — A finished mutmut result maps killed and timeout to detected, survived and no-tests to undetected, and a type-check rejection to left out.

2026-10-28 — A mutant with no finished outcome fails a touched function in that file. The score is not taken from the finished subset. A mutant the tool never generated stays absent.

2026-10-28 — Each language has its own mutation path, with no default. The config table is `[mutation]` with `python`, `rust`, and `cpp`. `--mutation-python`, `--mutation-rust`, and `--mutation-cpp` replace that language's key for one run, under the same path rules as `--coverage`. Unset is a missing report.

2026-10-28 — A run outside git uses the `--strict` placement. A missing report stays a note. A score below 1 is inherited and does not fail the verdict.

2026-10-28 — The rule id is S07. The gate is 1. CRAP is unchanged.

2026-10-28 — A mutation path that is set and cannot be parsed exits 2, including under `--strict` and outside git. A path that is unset, or set and missing on disk, stays the missing-report case.

## Answer

Resolved 2026-10-28 with Christopher. S07 gates the mutation score of a touched function at 1. The tool reads a report and does not run the mutator. CRAP stays on line coverage. A missing report fails the default run. `--strict` and a run outside git keep shown rows inherited. A waiver line for S07 fails the run. An unreadable report exits 2. Python reads `mutants/<file>.meta`, Rust reads `mutants.out/outcomes.json`, and C++ reads mull's mutation-testing-elements JSON. [ADR 0027](../../../docs/adr/0027-mutation-score-on-a-changed-function.md).

The readers are [Read a cargo-mutants report into S07](71-cargo-mutants-report.md), [Read a mutmut store into S07](72-mutmut-store.md), and [Read a mull report into S07](73-mull-report.md).
