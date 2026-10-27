# Load slopmeter.toml

Type: task
Status: resolved
Blocked by: 29

## Question

Implement session 3 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). A missing file is the built-in. The loader accepts a coverage path, include and exclude patterns, an F03 gate, and F08 allow-list names. An unknown key fails. Warn and gate for the other rules wait on [Which metric each rule id measures](34-rule-ids.md).

## Answer

`inputs::load_config(root)` reads `{root}/slopmeter.toml` and nothing lower in the tree. A missing file is the built-in profile: F03 warns above 10 and gates above 15 in every language, F07 stays at 80 with no key, and F08 stays at 2 hops from the receiver and 1 otherwise. The file may set `coverage`, `include`, `exclude`, an F03 `warn` or `gate`, and F08 `allow` names. `Config::retains` applies include, then exclude. `set_coverage_override` may be absolute. Other rule ids wait on the rule-id ticket. The code-level plan is [the config loader](../../../docs/superpowers/plans/2026-10-27-config-loader.md).
