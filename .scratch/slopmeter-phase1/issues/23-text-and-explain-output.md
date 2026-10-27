# Text table and explain output format

Type: prototype
Status: resolved
Blocked by: 21

## Question

Mock the human-facing outputs from the JSON report: the worst-first table (Rule · Symbol · File:line · Measured · Threshold, then `slop_score` and the verdict on the last line), the exit codes (0 pass, 1 usage error, 2 gate failed, and what a parse failure returns), and `slopmeter explain <rule>` (principle, counting method, threshold, two short before/after examples, anti-fix list, ≤ 1 page). Render both for the sloppy fixture and react to them.

## Answer

Resolved 2026-10-27 with Christopher. Why the table is the check a person reads, and why a missing check exits 2: [ADR 0019](../../../docs/adr/0019-text-table-and-explain.md). The renders are [prototype/text-output/](../../../prototype/text-output/).

A check prints the table on stdout. `--json` prints the report instead, with no table mixed in. The exit code is the same either way. A finished check exits 0 on pass and 1 when a gate finding fails it. The failing table stays on stdout.

A run that produces no check exits 2, writes the reason to stderr, and prints neither document. That is a wrong argument, an invalid config file, a diff that cannot be computed, or an unknown rule passed to `explain`. A partial parse still produces a check, and the exit code follows the verdict.

A row is the rule, `gate` or `warn`, the qualified name, `file:line`, and the measured value against its threshold, in report order. C++ parameter types stay on the JSON symbol. The principle, the fix shape, and the anti-fix stay off the row. Untouched findings sit under an `inherited` heading. An empty findings list prints no rows above it. An empty inherited list omits the heading. Notes sit above the last line and do not change the verdict. The last line is `slop_score` and the verdict. When the score is absent, the last line is the verdict alone.

`explain <rule>` prints one page on stdout and exits 0. The page is the principle, the counting method, the shipped threshold, one before and one after in source text, and the anti-fix list. It stays within a page. It does not read the project file, and it does not scan the repo. The finding's anti-fix sentence is the first line of that list. When the languages ship different numbers, the threshold section lists each language. When they share a number, it is one line. The before and after stay a single example. The words for the eight rules besides the F08 sample are not written yet.
