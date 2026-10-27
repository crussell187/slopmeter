# How a gate is waived

Type: grilling
Status: resolved
Blocked by: 34

## Question

Every function-scope gate can fail a run, and F06 fails at 20 physical lines. `slopmeter.toml` can raise a language-wide gate and cannot turn a rule off. There is no per-function exception. How does an agent waive any gate with a justification a person can read, and what does the tool require before that finding stops failing the run?

## Answer

Resolved 2026-10-27 with Christopher. Why a comment excuses one gate up to a stated measurement: [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md). The glossary terms are **Waiver**, **Gate**, **Touch span**, and **Finding** in `CONTEXT.md`.

A contiguous run of `#` or `//` comments sits directly above the touch span and joins it. The tokens are `slopmeter:`, `waive`, the rule id, and the number. The rest of the line is the reason. While the measurement is over the gate and at most that number, the finding stays a gate, keeps its weight, and the verdict ignores it. The table row shows the number and the reason. A line the tool cannot accept fails the check. A well-formed waiver under the gate does not. The Python adapter records the lines before the rule engine, in [Record waiver lines on the function](45-waiver-lines.md).
