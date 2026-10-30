# Reject a waiver the change adds and count the held ones

Type: task
Status: resolved
Blocked by: 

## Question

Build [ADR 0034](../../../docs/adr/0034-a-waiver-the-change-adds-does-not-hold.md). `scope` already holds each file's added line set and an untracked file's lines; a `WaiverLine` whose line is in that set is rejected on the default run with the rejection "written by this change", the metric finding fails, and the row shows the rejection. `--strict` and a run outside git hold every well-formed waiver. The JSON report gains `waived`, the count of findings whose waiver holds across both lists, next to `slop_score`, and the table's verdict line shows `waived N` above 0. The shared waiver sentence on each gate's explain page says the comment has to be in a change a person already reviewed. Goldens and the corpus tests cover a waiver on an added line, on an unchanged line, in an untracked file, and under `--strict`.

## Answer

Resolved 2026-10-30. On the default run, a waiver whose line is in the diff's added set does not hold, and every line of an untracked file is in that set. The metric finding stays, the row says the waiver was written by this change, and that finding fails the run. A waiver on an unchanged line holds. `--strict` and a run outside git hold every well-formed waiver. The JSON report carries `waived` next to `slop_score`: the findings whose waiver holds, across both lists. The table's verdict line shows `waived N` when that count is above 0. Each gate's explain page says the comment has to be in a change a person already reviewed. `slopmeter --strict` on this repo still exits 0.
