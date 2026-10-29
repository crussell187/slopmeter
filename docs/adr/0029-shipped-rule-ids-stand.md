---
status: accepted
date: 2026-10-28
---

# The shipped rule ids stand

The design spec numbered F04 as logical lines, F05 as parameters, F06 as nesting, and F11 as commented-out code. The tool ships F04 as parameter count, F05 as nesting, F06 as physical length, and F11 as logical lines. Those ids stay. Commented-out code, a comment block that would parse as code in the host language, is not a rule in this effort. F01, F02, F03, F07, and F08 were already the same metric in the spec and in the tool. F09 stays the unknown-rule id.

## Considered options

- **Renumber to the spec and implement commented-out code as F11.** Rejected. The calibrated gates, the corpus pins, the waivers, and the config keys are on the shipped ids. Once F11 is commented-out code, the spec has no id for physical length.
- **Leave commented-out code in the fog under a new id.** Rejected. This destination already names the function rules the tool ships. The rule returns only if the destination is redrawn.
- **Rewrite the explain pages, the goldens, and the external spec.** Rejected. The pages and the goldens already use the shipped ids. The external spec stays the historical source. This record is the correction.

## Consequences

- [Each function rule has one metric and a hard stand-in](0022-rule-ids-and-stand-in-thresholds.md) still names the metric on each id. The explain pages and the goldens are unchanged.
- [Fix the explain pages and the CLI surface](../../.scratch/slopmeter-phase1/issues/64-explain-and-cli-polish.md) still fixes threshold wording. It is not a renumber.
