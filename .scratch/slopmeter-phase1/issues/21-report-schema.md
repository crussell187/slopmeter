# Define the JSON report schema

Type: prototype
Status: open
Blocked by: 10, 11, 14

## Question

The spec's report-schema code block is empty, so this is unspecified. Write the JSON schema for `slopmeter check --json`: top-level run metadata (tool version, config hash, scope, language profiles), `findings[]` with the fields the finding-payload decision fixed, sort order (gate-first, then weight, then distance over threshold), `inherited[]`, per-module trust signals (unparsed ratio), `slop_score` and its denominator, and the gate verdict. Produce a sample report from a sloppy fixture and have the user react to it as an agent would: is the first finding the one to fix first, and does it say what, where, how much, why, and what shape the fix takes?
