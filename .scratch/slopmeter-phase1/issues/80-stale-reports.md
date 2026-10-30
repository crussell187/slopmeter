# Whether a stale coverage or mutation report holds

Type: grilling
Status: resolved
Blocked by: 

## Question

`inputs::lcov` and the three readers in `inputs::mutation` join a report to a function by file and line and never compare the report to the source. An agent that edits a function and reruns the check against the report from before the edit is scored by lines that moved, so an uncovered or unkilled line can land outside the new span and the touched function passes S06 and S07 on the old report. Decide what freshness the tool checks: the report's modification time against the scored file's, a source hash the tool writes beside the report, or the `--changed` base commit recorded in the report. Decide whether a stale report fails the touched function the way a missing one does, or is a note, and whether `--strict` and a run outside git care. The build slice follows the decision.

## Answer

Resolved 2026-10-29 with Christopher. Freshness is modification time. On the default run a report has to be newer than every file the diff changed, tests and untracked files included; a stale report is missing, so each touched function fails S06 or S07 and CRAP is absent for its files. Under `--strict` and outside git the comparison is per scored file, with a note and no row for a newer file. A tie is fresh and one note names the report and the newest file. [ADR 0035](../../../docs/adr/0035-a-report-older-than-the-change-is-missing.md). The build slice is [Treat a report older than the change as missing](86-stale-report-check.md).

## Comments

2026-10-29 — Freshness is modification time. A report is stale for a scored file when that file was modified after the report was. No format the tool reads carries a date, and slopmeter does not produce the reports, so there is no stamp step.

2026-10-29 — Stale is missing, per file. A touched function in a file newer than the report fails S06 or S07 on the function line, with a note naming the report and the file. Under `--strict` and outside git the file gets the note and no row. F02 follows: CRAP is absent for a file newer than its coverage report. A file older than the report keeps its rows.

2026-10-29 — On the default run a report has to be newer than every file the diff changed, any extension, untracked files included; one newer file makes that whole report stale, and each touched function fails. Under `--strict` and outside git the comparison is per scored file the report matches. For mutmut, `mutants/<file>.meta` is that file's report time and the newest `.meta` is the store's time against the diff.

2026-10-29 — A tie is fresh. One note per stale report, naming the report and the newest file: "coverage.lcov is older than src/lib.rs".
