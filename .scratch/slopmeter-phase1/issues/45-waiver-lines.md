# Record waiver lines on the function

Type: task
Status: open
Blocked by: 44

## Question

Implement the waiver record from [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md). `adapters::python::parse` records every line in the contiguous waiver run above a function: the source line, the rule token, the number token when present, and the reason when present. The adapter does not apply a gate. A comment inside a body does not waive that function. A waiver above a nested function attaches to the nested function and still counts toward the enclosing function's length. Rust and C++ record the same shape when those adapters land.
