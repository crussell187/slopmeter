# Text table and explain output format

Type: prototype
Status: open
Blocked by: 21

## Question

Mock the human-facing outputs from the JSON report: the worst-first table (Rule · Symbol · File:line · Measured · Threshold, then `slop_score` and the verdict on the last line), the exit codes (0 pass, 1 usage error, 2 gate failed, and what a parse failure returns), and `slopmeter explain <rule>` (principle, counting method, threshold, two short before/after examples, anti-fix list, ≤ 1 page). Render both for the sloppy fixture and react to them.
