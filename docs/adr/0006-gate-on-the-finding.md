---
status: accepted
date: 2026-10-26
---

# The run fails on a finding, not on the score

A gate has to name the function to change. `slop_score` is one number for the whole scored set, so a ceiling on it tells an agent that the run failed and not what to edit. Warns already add to the score, and a ceiling would let a pile of them fail the build. The run fails when any Tier A finding is over its gate level. Between a warn level and a gate level the finding is a warn: it is in the report and in the score, and the exit code stays 0. The score is reported beside the verdict. Phase 1 has no ceiling and no config knob for one.

## Considered options

- **Fail when the score crosses a ceiling.** Rejected. The agent gets a number and no next edit.
- **Fail on either a finding or a ceiling.** Rejected for the same reason, and there is no calibrated ceiling yet.

## Consequences

- An inherited finding stays listed and does not gate. Which findings are allowed to gate, including one that got worse, stays on [Diff-scoped by default, inherited findings, and --strict](../../.scratch/slopmeter-phase1/issues/11-diff-scoped-default.md).
- The spec's F03 shape is the example: warn above 10, gate above 15.
