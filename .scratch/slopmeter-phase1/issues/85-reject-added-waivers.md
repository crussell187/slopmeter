# Reject a waiver the change adds and count the held ones

Type: task
Status: open
Blocked by: 

## Question

Build [ADR 0034](../../../docs/adr/0034-a-waiver-the-change-adds-does-not-hold.md). `scope` already holds each file's added line set and an untracked file's lines; a `WaiverLine` whose line is in that set is rejected on the default run with the rejection "written by this change", the metric finding fails, and the row shows the rejection. `--strict` and a run outside git hold every well-formed waiver. The JSON report gains `waived`, the count of findings whose waiver holds across both lists, next to `slop_score`, and the table's verdict line shows `waived N` above 0. The shared waiver sentence on each gate's explain page says the comment has to be in a change a person already reviewed. Goldens and the corpus tests cover a waiver on an added line, on an unchanged line, in an untracked file, and under `--strict`.
