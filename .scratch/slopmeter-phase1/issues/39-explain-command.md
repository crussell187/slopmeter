# Print one explain page

Type: task
Status: resolved
Blocked by: 36, 27

## Question

Implement session 12 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `explain` prints one shipped-threshold page and exits 0. An unknown rule exits 2. The page text comes from [Author the nine explain pages](27-explain-pages.md).

## Answer

`slopmeter explain <RULE>` prints that rule's shipped page and exits 0. The page is `explain::page`, including the shipped stand-in threshold, and the command does not read a repo. An unknown rule, including `f08`, writes `unknown rule <id>` to stderr and exits 2. No rule, or more than one argument, also exits 2 and prints nothing. Any other invocation still does not score a tree. The code-level plan is [Explain command](../../../docs/superpowers/plans/2026-10-27-explain-command.md).
