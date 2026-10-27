# How a gate is waived

Type: grilling
Status: open
Blocked by: 34

## Question

Every function-scope gate can fail a run, and F06 fails at 20 physical lines. `slopmeter.toml` can raise a language-wide gate and cannot turn a rule off. There is no per-function exception. How does an agent waive any gate with a justification a person can read, and what does the tool require before that finding stops failing the run?
