---
status: accepted
date: 2026-10-26
---

# A finding reports the worst site, and does not plan the edit

The agent acts on the finding it is given, so the finding has to carry the counter and the measured facts, and it must not invent a refactor. One finding is the worst violation of one rule at one function. It carries the rule, the function's identity, the line of that site, the measured value, the one threshold it is over, the level `gate` or `warn`, the rule's weight, a one-sentence principle with its source, a fix shape, and one anti-fix sentence. The fix shape names the refactoring and fills in the measured value, that threshold, and the function's location. The anti-fix sentence names the cheapest edit that leaves the finding in place, and the same sentence is used for every finding of that rule.

## Considered options

- **Save anti-fix for Phase 3, or keep it in `explain` only.** Rejected. The finding is what the agent reads. A counter that lives only in a separate command is invisible on the run the agent has to pass.
- **Predict the result of the edit.** Rejected. A line range and a "drops to ≈ 6" ask the tool to plan the change. Phase 1 measures the function as written, and a wrong plan sends the agent to the wrong edit.
- **One finding per chain.** Rejected. The symbol is the function. A second over-limit site in the same function appears on the next run, after the worst one is gone.

## Consequences

- A function at cyclomatic complexity 18 carries threshold 15 and level `gate`. At 12 it carries threshold 10 and level `warn`. F07 is always `warn`, threshold 80. F08 is `gate` at the depth it crossed.
- An inherited finding keeps that level and threshold. The list it sits in stays on [Define the JSON report schema](../../.scratch/slopmeter-phase1/issues/21-report-schema.md).
- `explain` carries the longer anti-fix list, and the finding's sentence is its first line. The page format is [The text table is the check a person reads](0019-text-table-and-explain.md). The weight numbers are [A finding's weight is fixed for its rule and level](0025-finding-weight-is-fixed.md).
- A finding with a waiver line attached also carries the excused measurement and the reason. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md).
