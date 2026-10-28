# A coverage gate on changed lines

Type: grilling
Status: resolved
Blocked by: 59

## Question

F02 emits nothing without an LCOV report and there is no coverage-fraction rule, so an agent that never produces coverage is never gated on it (review, coverage section). Chris wants 100% coverage enforced. Decide S06: the metric is hit `DA` lines over `DA` lines inside touched functions' spans, diff-scoped like every other rule; the gate level (100%, or the spec's 80 warn / 90 gate); whether `BRDA` branch records count too, since 100% line coverage with no branch coverage is cheap; whether a missing report fails the default run instead of leaving a note; what happens to a touched function with no `DA` lines (a declaration, a macro); and the explain page and anti-fix (a test that runs everything and asserts nothing, which is the spec's S05). Then build it on the existing join in `src/inputs/lcov.rs`.

## Answer

Resolved 2026-10-28 with Christopher. S06 gates the coverage fraction CRAP already uses, hit `DA` lines over `DA` lines in the body span, at 100% with no warn. Branch records stay out. A touched function fails when the fraction is below 1, when its file has no matching record, or when the records tie. The finding sits on the first missed line, or on the function line when no fraction was produced. A matched span with no `DA` lines stays off the gate. An untouched function below 1 is inherited. An untouched function whose fraction was never produced gets no row. A missing report fails the default run once a function is touched, and the note stays. Under `--strict` a missing report adds no row and every shown S06 row stays inherited, so the score matches and coverage does not fail that run. A waiver line that names S06 fails the run. The anti-fix is a test that runs the function and asserts nothing. [ADR 0026](../../../docs/adr/0026-coverage-gate-on-a-changed-function.md).
