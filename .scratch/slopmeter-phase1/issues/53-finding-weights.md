# Finding weights and what slop_score means

Type: grilling
Status: resolved
Blocked by: 

## Question

Every weight is hard-coded 0 at `src/rules.rs:283` and `:309`, so `slop_score` is always 0.0 and weight-sorting is a no-op; ADR 0010 and 0017 deferred the numbers to the validation run, which never set them (review finding 11). The spec said gate 10 and warn 1–3 by rule. Decide the weights per rule and level, whether the score stays per physical KLOC, and whether the score should be dropped from the report until an agent loop can use it. Set the numbers in the rule engine and update the prototype and goldens.

## Answer

Resolved 2026-10-27 with Christopher. Why the weights are fixed: [ADR 0025](../../../docs/adr/0025-finding-weight-is-fixed.md). The glossary term **Slop score** already said this, so `CONTEXT.md` is unchanged.

`slop_score` stays in the report. It is the sum of every finding's weight, inherited findings included, divided by the thousands of physical lines in the scored set. A diff run and `--strict` score the same tree. The score does not fail the run. The weight is a fixed number for the rule and the level. Every gate weighs 10. F03's warn weighs 3. F07 weighs 1. A waiver that holds keeps that weight. A rejected waiver line that stands as its own finding weighs 0. `rules::evaluate` sets these numbers. The sample report and the rule-file goldens use them. The text table still does not show the weight.
