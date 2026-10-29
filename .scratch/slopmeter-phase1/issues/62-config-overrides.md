# Which gates the config may move, and how the report shows it

Type: grilling
Status: resolved
Blocked by: 

## Question

`slopmeter.toml` accepts only F03 levels and the F08 allow-list (`src/inputs/config.rs:299`) while ADR 0018 and 0022 describe per-rule gate departures, and F01, F04, F05, F06 and F11 are compile-time constants, so a project cannot tune them. At the same time an agent can set the F03 gate to any u32 and nothing in the report records it, and a `warn` set alone resets that language's gate to the generic 15 at `:335-336` (review finding 10). Decide: which rules accept `gate` and `warn`; whether a raised gate is bounded (at most twice the built-in) or unbounded; and whether every departure from the built-in profile is listed as a run-level note in the report so a config edit is visible to a reviewer. Fix the warn-only reset either way.

## Comments

2026-10-28 — A `gate` key is legal on F01, F02, F03, F04, F05, F06, and F11. A `warn` key stays legal only on F03. F07 has no keys. F08 keeps `allow` only. S06 and S07 take neither key.

2026-10-28 — A count gate may be lowered to 0 and raised to twice that language's built-in. A number above that fails before scoring. F03's warn has to sit strictly below the gate after the merge.

2026-10-28 — F03's warn may run from 0 to 20. F03's gate floor is 1. The other count gates may still be 0.

2026-10-28 — An omitted gate stays the language's built-in. A warn-only table does not reset the gate to 15.

2026-10-28 — Each changed gate, the changed warn, and each language's added allow-list names become one run-level note. The note names the language, the rule, the built-in, and the value in effect. A value equal to the built-in adds no note. Coverage, include, exclude, and the mutation paths are not departures. The note does not change the verdict.

## Answer

Resolved 2026-10-28 with Christopher. A count gate may rise to twice that language's built-in. F03's warn tops out at 20 and its gate floor is 1. Each departure is a run-level note and does not fail the run. [ADR 0028](../../../docs/adr/0028-count-gate-twice-the-built-in.md).

The build slice is [Accept a bounded count gate and note the departure](74-bounded-gate-overrides.md).
