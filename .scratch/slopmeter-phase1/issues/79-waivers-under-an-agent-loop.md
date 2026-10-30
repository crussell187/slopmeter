# Waivers under an agent loop

Type: grilling
Status: open
Blocked by: 

## Question

[ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md) caps a waiver's number and relies on a person reading the reason in the diff. In the loop the destination names, the agent under review writes the diff, so it can add `// slopmeter: waive F01 40 reason` above the function it just failed and pass; `rules::evaluate` holds the waiver, the finding keeps its weight, and the verdict ignores it. There is no switch to refuse waivers and the run-level notes do not count them. Decide whether a waiver on a line the diff adds or changes holds in that run, whether the loop needs a `--no-waivers` flag or a config key that refuses every waiver, whether the report counts held waivers at run level so a harness can gate on zero, and whether a waiver's number may exceed twice the gate the way the config ceiling does. The build slice follows the decision.
