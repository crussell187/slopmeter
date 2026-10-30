# Bring this crate under the gates

Type: task
Status: open
Blocked by:

## Question

`slopmeter --strict` on this repo, with `corpus/` excluded, exits 1. Measured 2026-10-29: 364 gate findings, 357 in `src/` and 7 in `prototype/ir-schema/`. Change this crate until that run exits 0. The bar is the tool's own exit code. There is no baseline of those findings, and the config does not raise gates to force a pass. [The self-check joins CI when this repo passes](../../../docs/adr/0033-self-check-when-the-repo-passes.md). When the run exits 0, [Put the self-check in CI and create the GitHub remote](65-self-check-and-remote.md) can add the command to the workflow.
