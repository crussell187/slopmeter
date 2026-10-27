# Finding weights and what slop_score means

Type: grilling
Status: open
Blocked by: 

## Question

Every weight is hard-coded 0 at `src/rules.rs:283` and `:309`, so `slop_score` is always 0.0 and weight-sorting is a no-op; ADR 0010 and 0017 deferred the numbers to the validation run, which never set them (review finding 11). The spec said gate 10 and warn 1–3 by rule. Decide the weights per rule and level, whether the score stays per physical KLOC, and whether the score should be dropped from the report until an agent loop can use it. Set the numbers in the rule engine and update the prototype and goldens.
