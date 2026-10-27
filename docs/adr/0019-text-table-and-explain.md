---
status: accepted
date: 2026-10-27
---

# The text table is the check a person reads

The JSON report is what an agent clears. A person running the command gets the same check as a table: one row per finding, in report order, then notes, then `slop_score` and the verdict. `--json` prints the report instead, and the exit code does not change. A finished check exits 0 on pass and 1 on fail. A run that produces no check exits 2.

## Considered options

- **Exit 1 for a usage error and 2 for a failed gate.** Rejected. The report decision already exits 1 when a gate finding fails the check. A second number for that failure would split one verdict across two codes.
- **Print the table and the JSON together.** Rejected. An agent that reads stdout would have to find the document inside the table.
- **Put the project's overridden threshold on `explain`.** Rejected. The command is the rule's documentation and does not need a repo. The finding already carries the threshold that function crossed.

## Consequences

- A row is the rule, `gate` or `warn`, the qualified name, `file:line`, and the measured value against its threshold. C++ parameter types stay on the JSON symbol. The principle, the fix shape, and the anti-fix stay off the row.
- Untouched findings sit under an `inherited` heading. An empty findings list prints no rows above it. An empty inherited list omits the heading. A note sits above the last line and does not change the verdict. When the score is absent, the last line is the verdict alone.
- The table, and the JSON report, go to stdout, including when the check fails. A run with no check writes the reason to stderr and prints neither. That run is a wrong argument, an invalid config file, a diff that cannot be computed, or an unknown rule passed to `explain`. A partial parse still produces a check.
- `explain <rule>` prints one page on stdout and exits 0. The page is the principle, the counting method, the shipped threshold, one before and one after in source text, and the anti-fix list. The finding's anti-fix sentence is the first line of that list. When the languages ship different numbers, the threshold section lists each language. When they share a number, it is one line. The before and after stay a single example. The page does not read the project file.
- The sample table is `prototype/text-output/`. The nine shipped pages are `docs/explain/`.
- A row with a waiver also shows the excused number and the reason. Every gate's explain page carries the shared waiver sentence. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md).
