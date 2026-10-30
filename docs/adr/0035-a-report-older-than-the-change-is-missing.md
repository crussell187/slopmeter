---
status: accepted
date: 2026-10-29
---

# A report older than the change is missing

The coverage join and the three mutation readers match a report to a function by file and line. An agent that edits and reruns the check against the report from before the edit is scored by lines that moved, and a test it deleted after the report was written still counts as run. A stale report is a missing one. On the default run a report has to be newer than every file the diff changed, any extension, untracked files included. One newer file makes that whole report stale, each touched function fails S06 or S07 on its function line, and CRAP is absent for every file the report covers. Under `--strict` and outside git there is no diff, so the comparison is per scored file: a file modified after the report gets a note and no row, and a file older than the report keeps its rows. Freshness is the file system's modification time. A tie is fresh. One note per stale report names the report and the newest file.

## Considered options

- **A content hash slopmeter writes beside the report.** Rejected. Slopmeter does not produce the reports, so the hash would need a stamp step a harness can forget, and a step the harness forgets is a report that never goes stale.
- **The diff base commit recorded in the report.** Rejected. LCOV, mull's JSON, and mutmut's store record no commit, and cargo-mutants writes its start time to `lock.json`, not to `outcomes.json`.
- **Compare only the scored files the report matches on the default run.** Rejected. A deleted test is not a scored file and moves coverage in a file that was not edited.
- **A note that does not fail the run.** Rejected. A missing report fails a touched function, and a stale report is the same report after an edit.

## Consequences

- The check reads modification times of the report, the diff's changed paths, and under `--strict` the scored files. A branch switch can make a valid report look stale; the cost is one rerun.
- For mutmut, `mutants/<file>.meta` is that file's report time, and the newest `.meta` is the store's time against the diff.
- [The coverage gate on a changed function](0026-coverage-gate-on-a-changed-function.md) and [the mutation score on a changed function](0027-mutation-score-on-a-changed-function.md) each gain the stale case beside the missing one. The check is [Treat a report older than the change as missing](../../.scratch/slopmeter-phase1/issues/86-stale-report-check.md).
