---
status: accepted
date: 2026-10-29
---

# A waiver the change adds does not hold

A waiver is a comment a person reads in the diff. In the loop the tool exists for, the agent writes the diff, so a waiver written in the same change is the agent excusing itself. A waiver line in the diff's added set does not hold in that run. The gate finding fails as if the line were absent, and the row says the waiver was written by this change. A waiver on an unchanged line holds as before. Under `--strict` and outside git there is no diff, so every well-formed waiver holds. The JSON report carries one run-level integer, `waived`, next to `slop_score`: the findings whose waiver holds across `findings` and `inherited`. The table's verdict line shows `waived N` when that number is above 0. The waiver number has no cap, and there is no switch or config key that refuses waivers.

## Considered options

- **A `--no-waivers` flag, or a config key, that refuses every waiver.** Rejected. The harness can forget the flag, and the same tree would have two verdicts, one for the loop and one for a person running the same command. The diff rule needs nothing passed.
- **Cap the waiver number at twice the gate, the config ceiling.** Rejected. The ceiling moves a whole language. A waiver moves one function with a stated reason, and the functions that honestly need one, a generated table or a wide `match`, are often far past twice the gate.
- **Hold a waiver the change adds and count it.** Rejected. A count a harness may ignore does not gate. The verdict is the gate.
- **A note per held waiver.** Rejected. The row already shows the excused number and the reason. One integer is what a harness gates on.

## Consequences

- A rejected waiver on an added line is a rejection like the malformed ones in [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md): the finding stays the metric finding when the metric is over its gate, the row names the rejection, and the run exits 1. A waiver on an added line whose metric is under the gate is not a finding and does not fail the run.
- An untracked file has every line added, so a waiver in a new file never holds on the default run. Moving a function moves its waiver into the added set and re-exposes the gate once.
- Rejected waivers are not counted in `waived`; each one already fails the run.
- The explain pages' shared waiver sentence says the comment has to be in a change a person already reviewed. The count is [Reject a waiver the change adds and count the held ones](../../.scratch/slopmeter-phase1/issues/85-reject-added-waivers.md).
