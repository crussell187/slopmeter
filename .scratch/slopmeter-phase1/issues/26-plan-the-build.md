# Plan the Phase 1 build

Type: task
Status: resolved
Blocked by: 25

## Question

The decisions are frozen: the IR, the adapter contract, the report, the config file, the text table and `explain` page, and the counting rules. Write the implementation plan for the one crate. The first slice is the core, the Python adapter, and the LCOV reader, and it ends in the Python validation run. The Rust adapter and its run are next. The C++ adapter and its run are last. Place the fixture tests, the reporter golden files, and the cyclomatic-complexity cross-checks in that plan. Each slice has to fit one session.

## Answer

Resolved 2026-10-27 with the writing-plans skill. The plan is [docs/superpowers/plans/2026-10-27-phase1.md](../../../docs/superpowers/plans/2026-10-27-phase1.md).

Session 1 is the crate and the IR types, with the fmt, clippy, and test workflow. Its tasks include the code. Sessions 2–16 are one ticket each, in order: source selection, the config loader, Python functions, Python classification, analyzers, the rule-id decision, the rule engine, reporters, diff scope, LCOV and CRAP, the explain command, fixtures and the cyclomatic-complexity cross-check, the Python validation run, the Rust adapter and its run, and the C++ adapter and its run.

F01, F02, F04, F05, F06, and F11 are not assigned to metrics in the decisions so far. [Which metric each rule id measures](34-rule-ids.md) blocks the rule engine and the explain pages. F03, F07, and F08 stay as already decided.
