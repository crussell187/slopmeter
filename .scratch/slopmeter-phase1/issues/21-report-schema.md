# Define the JSON report schema

Type: prototype
Status: resolved
Blocked by: 10, 11, 14

## Question

The spec's report-schema code block is empty, so this is unspecified. Write the JSON schema for `slopmeter check --json`: top-level run metadata (tool version, config hash, scope, language profiles), `findings[]` with the fields the finding-payload decision fixed, sort order (gate-first, then weight, then distance over threshold), `inherited[]`, per-module trust signals (unparsed ratio), `slop_score` and its denominator, and the gate verdict. Produce a sample report from a sloppy fixture and have the user react to it as an agent would: is the first finding the one to fix first, and does it say what, where, how much, why, and what shape the fix takes?

## Answer

Resolved 2026-10-26 with Christopher. Why the report splits touched findings from inherited ones, and why the score does not fail the run: [ADR 0017](../../../docs/adr/0017-json-report-lists-what-must-be-cleared.md). The sample is [prototype/report-schema/report.json](../../../prototype/report-schema/report.json).

`findings` holds every finding in a touched function, gate or warn. `inherited` holds every finding in a function the change did not touch, and that finding keeps its level. The verdict is `fail` when `findings` contains a level `gate`, and `pass` otherwise. The process exits 1 on `fail` and 0 on `pass`. A warn does not have to be cleared for the verdict to pass. An empty diff passes, and every finding is in `inherited`.

Within each list, gate entries come before warns, then higher weight, then a larger `(measured_value - threshold) / threshold`. Ties break by path, then by line. The array order is the rank.

`slop_score` sums both lists' weights and divides by the thousands of physical lines in the scored set. `kloc` is that denominator. The score is null when there are no lines. The same tree scores the same on a diff run and on `--strict`. The score does not change the verdict.

A finding carries `rule`, `level`, `symbol` (`path`, `name`, `param_types`, `qualifier`), `line`, `measured_value`, `threshold`, `weight`, `principle`, `fix_shape`, and `anti_fix`. `principle`, `fix_shape`, and `anti_fix` are one string each. The line is the site already decided: the chain, the forwarding call, or the `def` or `fn` line.

The top of the document also carries `scope` (`changed` or `strict`) and `tool_version`. There is no config hash and no copy of the threshold profile.

`notes` are `{path, message}`. `path` is absent on a run-level note. Notes do not change the verdict or the score. A scored file is noted when its unparsed ratio is above 0, when its coverage record is missing or tied, or when a function in it has no `DA` lines. A run with no coverage report is one note and no path. Files at ratio 0 are omitted.

This document is a finished check. A run that cannot compute the diff does not print it.
