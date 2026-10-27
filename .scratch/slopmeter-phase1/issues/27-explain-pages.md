# Author the nine explain pages

Type: task
Status: resolved
Blocked by:

## Question

The page format is frozen. Write the principle, the counting method, the shipped threshold, one before and one after, and the anti-fix list for F01–F08 and F11. Each page stays within a page. The finding's anti-fix sentence is the first line of that list. When the languages ship different numbers, the threshold section lists each language. The words have to match the counting rules already decided, including cognitive complexity and logical lines. Threshold numbers that calibration has not set stay marked as the shipped stand-in. Every gate's page also carries the shared waiver sentence from [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md): the comment shape, the number, and the reason. F07's page says a waiver line for it fails the run.

## Answer

`explain::page` returns the shipped page for F01–F08 and F11, and nothing for any other id. The pages are [docs/explain/](../../../docs/explain/). Each gate page carries the same waiver sentence: the comment shape, the number, and the reason. F07 says a waiver line for it fails the run. Every threshold is marked as the shipped stand-in. F02 says CRAP grows with cyclomatic complexity and uncovered lines, and states no algebraic formula; that formula is still [Join coverage and compute CRAP](38-lcov-crap.md). The code-level plan is [Explain pages](../../../docs/superpowers/plans/2026-10-27-explain-pages.md).
