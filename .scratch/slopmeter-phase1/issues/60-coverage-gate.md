# A coverage gate on changed lines

Type: grilling
Status: open
Blocked by: 59

## Question

F02 emits nothing without an LCOV report and there is no coverage-fraction rule, so an agent that never produces coverage is never gated on it (review, coverage section). Chris wants 100% coverage enforced. Decide S06: the metric is hit `DA` lines over `DA` lines inside touched functions' spans, diff-scoped like every other rule; the gate level (100%, or the spec's 80 warn / 90 gate); whether `BRDA` branch records count too, since 100% line coverage with no branch coverage is cheap; whether a missing report fails the default run instead of leaving a note; what happens to a touched function with no `DA` lines (a declaration, a macro); and the explain page and anti-fix (a test that runs everything and asserts nothing, which is the spec's S05). Then build it on the existing join in `src/inputs/lcov.rs`.
