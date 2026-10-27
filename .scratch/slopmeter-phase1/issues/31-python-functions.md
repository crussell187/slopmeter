# Parse Python functions

Type: task
Status: resolved
Blocked by: 30

## Question

Implement session 4 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `adapters::python::parse` returns one file. A nested function is its own record. A lambda inside a function is not. A lambda bound to a name outside any function is.

## Answer

`adapters::python::parse(path, source)` returns one `File` and does not open the path. The acceptance fixture records `outer`, `outer.inner`, and `bound`: the enclosing `def` is a function, the nested `def` is its own record, the lambda inside `inner` is not, and the module-level `bound = lambda` is. A method's first parameter is the receiver, `@staticmethod` has none, and `@overload` is skipped. Decorators extend the touch span. A latin-1 cookie is decoded. `ALLOW` is the 23 Python member names. Logical lines, decisions, cognitive increments, chains, and `forward` stay empty until [Classify a Python body](32-python-classification.md). The code-level plan is [Python functions](../../../docs/superpowers/plans/2026-10-27-python-functions.md).
