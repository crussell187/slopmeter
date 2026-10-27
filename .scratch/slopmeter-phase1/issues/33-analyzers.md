# Derive the measurements

Type: task
Status: resolved
Blocked by: 32

## Question

Implement session 6 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `analyzers::measure` turns one file into cyclomatic complexity, cognitive complexity, nesting, parameter count, chain depth, and the forward ratio.

## Answer

`analyzers::measure(file, allow)` returns one measurement per function, in file order. Cyclomatic complexity is 1 plus the decision count. Cognitive complexity sums 1 plus depth for a structural increment and 1 for a hybrid or fundamental increment. Nesting is the deepest decision depth, or 0. Parameter count leaves out the receiver, and that count is the denominator of the forward ratio. The numerator is the forwarded names; with no forwarding call the measurement has no forward. Chain depth drops allow-list names the caller supplies, and a trailing run of calls that ends in `build`. A rooted chain and any other chain are kept apart. Each side keeps the greatest depth, an equal depth keeps the earlier line, and depth 0 is absent. The code-level plan is [Analyzers](../../../docs/superpowers/plans/2026-10-27-analyzers.md).
