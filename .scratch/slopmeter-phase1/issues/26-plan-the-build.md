# Plan the Phase 1 build

Type: task
Status: open
Blocked by: 25

## Question

The decisions are frozen: the IR, the adapter contract, the report, the config file, the text table and `explain` page, and the counting rules. Write the implementation plan for the one crate. The first slice is the core, the Python adapter, and the LCOV reader, and it ends in the Python validation run. The Rust adapter and its run are next. The C++ adapter and its run are last. Place the fixture tests, the reporter golden files, and the cyclomatic-complexity cross-checks in that plan. Each slice has to fit one session.
