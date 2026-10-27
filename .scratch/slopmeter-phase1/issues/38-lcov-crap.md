# Join coverage and compute CRAP

Type: task
Status: resolved
Blocked by: 36

## Question

Implement session 11 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). Coverage is hit `DA` lines over `DA` lines in the body span. A missing report leaves CRAP uncomputed and adds the run-level note. `--coverage` replaces the file's path.

## Answer

`inputs::read_lcov` keeps `DA` lines and ignores function records. `inputs::cover` joins those lines to the body span: hit lines over listed lines, with a nested function's lines taken out of the parent. CRAP is cyclomatic complexity squared, times the uncovered fraction cubed, plus cyclomatic complexity. No report leaves CRAP unset and adds `No coverage report was given. CRAP was not computed.` A missing file, a tie, and a function with no `DA` lines are notes and do not gate. `--coverage` replaces the file key, and a repo-relative key is read from the root. The binary does not score a tree yet. The code-level plan is [LCOV and CRAP](../../../docs/superpowers/plans/2026-10-27-lcov-crap.md).
