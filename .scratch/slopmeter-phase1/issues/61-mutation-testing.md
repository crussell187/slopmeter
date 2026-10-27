# Mutation testing as a slopmeter gate

Type: grilling
Status: open
Blocked by: 55, 60

## Question

The spec listed mutation testing as a v1 non-goal and scheduled S07 (mutation score on changed functions, CRAP′ = CC² × (1 − mutation)³ + CC) for Phase 3; the map put it out of scope and nothing in the tool reads a mutation report. Chris wants it in the chain. Decide the shape: the tool reads a report, as with coverage, and does not run the mutation tool; per language the inputs are cargo-mutants' `mutants.out/outcomes.json` (caught, missed, timeout, unviable per mutant; `--in-diff` matches diff scoping), mutmut 3's `mutants/` store or `export-cicd-stats` (no documented per-mutant JSON), and for C++ mull's mutation-testing-elements JSON or dextool, both of which need a compilation database, which the C++ macro-policy decision governs. Decide the gate level (spec: 60 gate / 80 warn), whether mutation replaces coverage in CRAP when present, how a mutant maps to a function (file and line against the body span), what a function with no mutants scores, and whether C++ gets S07 at all in this effort. The per-language report readers are build slices once this is decided.
