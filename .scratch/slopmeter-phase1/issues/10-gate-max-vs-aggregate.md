# Gate on the max finding or on slop_score

Type: grilling
Status: resolved
Blocked by: 

## Question

The spec's open question: does the run fail on any single Tier A finding over its gate threshold (crap4java style), on the aggregate `slop_score` crossing a configured ceiling, or on both? Max is easier to explain to a model; aggregate is harder to game but harder to act on. The spec leans: gate on max for Tier A rules, report the aggregate. Decide, and decide whether `slop_score` ceilings are configurable in Phase 1 at all.

## Answer

Resolved 2026-10-26 with Christopher. Why the run fails on a finding: [ADR 0006](../../../docs/adr/0006-gate-on-the-finding.md).

The run fails when any Tier A finding is over its gate level. It does not fail on `slop_score`. The score is reported beside the verdict. Phase 1 has no ceiling and no config knob for one. A Tier A rule with two levels uses the lower one as a warn: the finding is in the report and in the score, and the exit code stays 0. The spec's F03 defaults are the example: warn above 10, gate above 15. A function at 12 warns. A function at 16 fails the run.

An inherited finding stays listed and does not gate. Which findings are allowed to gate, including one that was already over the line and got worse, stays on [Diff-scoped by default, inherited findings, and --strict](11-diff-scoped-default.md).
