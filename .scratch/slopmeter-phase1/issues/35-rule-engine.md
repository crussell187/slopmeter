# Turn measurements into findings

Type: task
Status: open
Blocked by: 33, 34, 45

## Question

Implement session 8 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `rules::evaluate` emits one finding per rule per function at the worst site. Levels and thresholds come from the built-in profile. F07 stays warn-only. F08 stays gate-only. A waiver recorded on the function is honored as [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md) describes: a held waiver stays a gate the verdict ignores, and a rejected line fails the check.
