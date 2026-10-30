# Whether a stale coverage or mutation report holds

Type: grilling
Status: open
Blocked by: 

## Question

`inputs::lcov` and the three readers in `inputs::mutation` join a report to a function by file and line and never compare the report to the source. An agent that edits a function and reruns the check against the report from before the edit is scored by lines that moved, so an uncovered or unkilled line can land outside the new span and the touched function passes S06 and S07 on the old report. Decide what freshness the tool checks: the report's modification time against the scored file's, a source hash the tool writes beside the report, or the `--changed` base commit recorded in the report. Decide whether a stale report fails the touched function the way a missing one does, or is a note, and whether `--strict` and a run outside git care. The build slice follows the decision.
