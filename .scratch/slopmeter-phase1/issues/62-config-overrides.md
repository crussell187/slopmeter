# Which gates the config may move, and how the report shows it

Type: grilling
Status: open
Blocked by: 

## Question

`slopmeter.toml` accepts only F03 levels and the F08 allow-list (`src/inputs/config.rs:299`) while ADR 0018 and 0022 describe per-rule gate departures, and F01, F04, F05, F06 and F11 are compile-time constants, so a project cannot tune them. At the same time an agent can set the F03 gate to any u32 and nothing in the report records it, and a `warn` set alone resets that language's gate to the generic 15 at `:335-336` (review finding 10). Decide: which rules accept `gate` and `warn`; whether a raised gate is bounded (at most twice the built-in) or unbounded; and whether every departure from the built-in profile is listed as a run-level note in the report so a config edit is visible to a reviewer. Fix the warn-only reset either way.
