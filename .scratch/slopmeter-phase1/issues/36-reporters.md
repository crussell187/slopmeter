# Print the table and the JSON report

Type: task
Status: resolved
Blocked by: 35

## Question

Implement session 9 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). The default run prints the text table on stdout. `--json` prints the report and no table. A finished check exits 0 or 1. A run that produces no check exits 2. A row with a waiver also shows the excused number and the reason, as [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md) describes.

## Answer

`reporters::render(report, json)` sorts each list, scores both, and returns the table or the JSON document with exit 0 or 1. A gate in `findings` fails unless its waiver holds. An inherited gate does not. The table shares column widths, and a waiver row shows the excused number and the reason, or the rejection sentence. `reporters::no_check` writes the reason to stderr and exits 2. The binary does not score a tree yet. The code-level plan is [Reporters](../../../docs/superpowers/plans/2026-10-27-reporters.md).
