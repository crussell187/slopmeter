# Turn measurements into findings

Type: task
Status: resolved
Blocked by:

## Question

Implement session 8 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `rules::evaluate` emits one finding per rule per function at the worst site. Levels and thresholds come from the built-in profile. F07 stays warn-only. F08 stays gate-only. A waiver recorded on the function is honored as [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md) describes: a held waiver stays a gate the verdict ignores, and a rejected line fails the check.

## Answer

`rules::evaluate(measurements, profile)` returns one finding per rule per function, in rule order, and a rejected waiver line follows when that rule did not already produce a gate. A value above the stand-in fails and the stand-in itself passes. F03 warns above 10 and gates above 15. F07 warns at 80 percent. F08 is the chain that crossed further, and an equal fraction keeps the earlier line. Absent CRAP emits no F02. A waiver holds only for a gate whose measurement is at most the written number; the threshold on the finding stays the gate. A missing rule, a rule that is not a gate, a missing number or reason, a fraction on a whole-number gate, and the same rule twice are rejected. Weights stay 0 until the validation run. The code-level plan is [Rule engine](../../../docs/superpowers/plans/2026-10-27-rule-engine.md).
