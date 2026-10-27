# Classify a Python body

Type: task
Status: resolved
Blocked by: 31

## Question

Implement session 5 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `parse` fills logical lines, decision lines, cognitive increments, member chains, and the forwarding call. The counts match the cyclomatic, cognitive, and logical-line decisions.

## Answer

`adapters::python::parse` fills `lloc`, `decisions`, `cognitive`, `chains`, and `forward` on each function. `choose` has five logical lines: the `if`, the `return`, the lambda body, the assignment, and the final `return`. Its two decisions are the `if` and the `and`, both at depth 1. The cognitive increments are the `if`, the `and` run, and the `else`. `ship` is one forwarding call after its docstring. A nested `def` keeps its own counts, and a lambda's decision stays on the function that contains it, one nesting deeper. A `match` is one structural increment; each specified label, and a guard, is a decision; `_` and a comma sequence are one label or none. Member chains keep every hop, including allow-list names. A bare forwarding call is not also a chain. A dotted chain on that call stays. The code-level plan is [Python classification](../../../docs/superpowers/plans/2026-10-27-python-classification.md).
