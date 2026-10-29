---
status: accepted
date: 2026-10-27
---

# A finding's weight is fixed for its rule and level

`slop_score` stays in the report. It is the sum of every finding's weight, inherited findings included, divided by the thousands of physical lines in the scored set. A diff run and `--strict` score the same tree. The score does not fail the run. A weight of 0 made that field a constant and made weight-sorting a no-op, so the spec's "reduce `slop_score` below 5" had nothing to reduce.

The weight is a fixed number for the rule and the level. How far the measurement sits over the threshold orders the table, and it does not change the weight. Every gate weighs 10: F01, F02, F03 at gate level, F04, F05, F06, F08, F11, S06, and S07. F03's warn weighs 3. F07 weighs 1. One gate per two thousand lines is a score of 5. Three cognitive warns weigh 9, just under one gate. Ten pass-throughs weigh the same as one gate.

A waiver that holds keeps that weight. The score describes the tree, and the verdict describes the change. A rejected waiver line that stands as its own finding weighs 0, so a malformed line does not move the score.

## Considered options

- **Drop `slop_score` until an agent loop can use it.** Rejected. The field is already what the report shows beside the verdict. A constant 0 hides the pile of findings the loop is supposed to watch.
- **Scale the weight by the excess over the threshold.** Rejected. The table already orders a larger `(measured_value - threshold) / threshold` first. A cyclomatic complexity of 200 and one of 9 are each one gate.
- **Give each gate rule its own weight.** Rejected. The verdict already fails on any gate. A second ranking would tell the agent that one failing rule matters more than another, with no evidence for the order.
- **Give a held waiver weight 0.** Rejected. Waiving every gate would drive the score to 0 while the findings stayed in the report.

## Consequences

- The rule engine sets these numbers. [The config file records departures from the built-in profiles](0018-config-records-departures.md) still has no weight key.
- The text table does not show the weight. The JSON finding carries it, and the score line is the total.
- The sample report is `prototype/report-schema/report.json`. Its score is `(10 + 1 + 10) / 0.04`.
- [A finding reports the worst site, and does not plan the edit](0010-finding-payload.md) carries this weight. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md) still keeps it on a held gate.
