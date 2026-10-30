# Waivers under an agent loop

Type: grilling
Status: resolved
Blocked by: 

## Question

[ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md) caps a waiver's number and relies on a person reading the reason in the diff. In the loop the destination names, the agent under review writes the diff, so it can add `// slopmeter: waive F01 40 reason` above the function it just failed and pass; `rules::evaluate` holds the waiver, the finding keeps its weight, and the verdict ignores it. There is no switch to refuse waivers and the run-level notes do not count them. Decide whether a waiver on a line the diff adds or changes holds in that run, whether the loop needs a `--no-waivers` flag or a config key that refuses every waiver, whether the report counts held waivers at run level so a harness can gate on zero, and whether a waiver's number may exceed twice the gate the way the config ceiling does. The build slice follows the decision.

## Answer

Resolved 2026-10-29 with Christopher. A waiver line in the diff's added set does not hold in that run; the gate fails and the row says the waiver was written by this change. A waiver on an unchanged line holds, and under `--strict` or outside git every well-formed waiver holds. No switch or config key refuses waivers, and the number has no cap. The report carries one run-level `waived` count of held waivers, shown on the table's verdict line above 0. [ADR 0034](../../../docs/adr/0034-a-waiver-the-change-adds-does-not-hold.md). The build slice is [Reject a waiver the change adds and count the held ones](85-reject-added-waivers.md).

## Comments

2026-10-29 — A waiver line in the diff's added set does not hold in that run. The gate fails as if the line were absent, and the row says the waiver was written by this change. A waiver on an unchanged line holds. Under `--strict` and outside git every well-formed waiver holds.

2026-10-29 — No `--no-waivers` switch and no config key. One verdict per tree.

2026-10-29 — The JSON report carries one run-level integer, `waived`, next to `slop_score`: the findings whose waiver holds across both lists. The table's verdict line shows "waived N" when it is above 0. Rejected waivers are not counted; each already fails the run.

2026-10-29 — The waiver number has no cap. The config ceiling of twice the built-in moves a whole language; a waiver moves one function with a reason.
