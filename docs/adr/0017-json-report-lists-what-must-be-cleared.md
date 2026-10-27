---
status: accepted
date: 2026-10-26
---

# The JSON report lists what the agent must clear

The agent reads one document. `findings` holds every finding in a touched function, gate or warn. `inherited` holds every finding in a function the change did not touch. An inherited finding keeps its level. The verdict is `fail` when `findings` contains a level `gate`, and `pass` otherwise. The process exits 1 on `fail` and 0 on `pass`. A warn does not fail the run. An empty diff has a passing verdict, and every finding is in `inherited`.

Inside each list, gate entries come before warns, then higher weight, then a larger fraction `(measured_value - threshold) / threshold`. Ties break by path, then by line. The order is the array order.

`slop_score` sums the weight of every finding in both lists and divides by the thousands of physical lines in the scored set. The same tree scores the same on a diff run and on `--strict`. The score is null when that set has no lines. `kloc` is the denominator. The score does not change the verdict.

A finding carries `rule`, `level`, `symbol` (`path`, `name`, `param_types`, `qualifier`), `line`, `measured_value`, `threshold`, `weight`, `principle`, `fix_shape`, and `anti_fix`. The three sentences are one string each. The top of the document also carries `scope` (`changed` or `strict`) and `tool_version`.

`notes` are `{path, message}`, and `path` is absent on a run-level note. A note does not change the verdict or the score. A scored file is noted when its unparsed ratio is above 0, when its coverage record is missing or tied, or when a function in it has no `DA` lines. A run with no coverage report is one note and no path. Files at ratio 0 are omitted.

## Considered options

- **A third list for warns.** Rejected. A touched warn would sit with debt the change did not touch, or in a list the agent is not told to read.
- **Fail the verdict on an inherited gate.** Rejected. The diff is the baseline. An untouched function is listed so it can be seen, and it does not fail the run.
- **A config hash and a copy of the threshold profile.** Rejected. Each finding already carries the threshold it crossed. The config file's shape is a separate decision.

## Consequences

- This document is a finished check. The process exits 0 on pass and 1 on fail. A run that produces no check exits 2 and does not print it. The text rendering is [The text table is the check a person reads](0019-text-table-and-explain.md).
- A gate a waiver holds stays in its list and does not fail the verdict. A rejected waiver line does. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md).
- The sample is `prototype/report-schema/report.json`. Its weights, sentences, version, and cognitive-complexity number are stand-ins.
- Weight numbers and the logical-line count remain open.
