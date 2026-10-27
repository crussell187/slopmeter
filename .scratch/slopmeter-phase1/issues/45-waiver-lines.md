# Record waiver lines on the function

Type: task
Status: resolved
Blocked by: 44

## Question

Implement the waiver record from [ADR 0023](../../../docs/adr/0023-a-waiver-excuses-one-gate.md). `adapters::python::parse` records every line in the contiguous waiver run above a function: the source line, the rule token, the number token when present, and the reason when present. The adapter does not apply a gate. A comment inside a body does not waive that function. A waiver above a nested function attaches to the nested function and still counts toward the enclosing function's length. Rust and C++ record the same shape when those adapters land.

## Answer

`parse` stores each attached line on `Function.waivers` as `WaiverLine { line, rule, number, reason }`. A missing rule, number, or reason is absent, and the line still stays in the run. A blank line or any other comment ends the run. The lines join `touch` and stay outside `span`. A comment inside a body does not waive that function. A waiver above a nested function attaches to the nested function; that line lies in the enclosing body span. A named lambda keeps the waiver above its assignment. The code-level plan is [Waiver lines](../../../docs/superpowers/plans/2026-10-27-waiver-lines.md).
