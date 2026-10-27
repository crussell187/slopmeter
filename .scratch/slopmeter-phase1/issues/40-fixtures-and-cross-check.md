# Lock the reporters and the CC delta

Type: task
Status: resolved
Blocked by: 37, 38, 39

## Question

Implement session 13 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). One fixture file per rule. Golden files for the text table and the JSON report. A Python cyclomatic-complexity fixture records the allowed delta against radon.

## Answer

`tests/rules` has one Python file per rule. Scored together, strict, each file's finding is only that rule. `tests/rules/coverage.lcov` gives F02 coverage 0, so its CRAP is 12, and the other eight files are noted as missing from the report. `tests/rules/table.txt` and `tests/rules/report.json` are the rendered check. `tests/cc/decisions.py` records cyclomatic complexity against radon 6.0.1. The only non-zero delta is `or_pattern`, 4 against radon's 2. `with` is 0 on both sides. The binary still does not score a tree. The code-level plan is [Fixtures and the CC cross-check](../../../docs/superpowers/plans/2026-10-27-fixtures-and-cross-check.md).
