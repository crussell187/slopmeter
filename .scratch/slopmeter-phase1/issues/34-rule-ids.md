# Which metric each rule id measures

Type: grilling
Status: resolved
Blocked by:

## Question

F03 is cognitive complexity, F07 is the pass-through warn, and F08 is the member-chain gate. The spec's other function rules are F01, F02, F04, F05, F06, and F11, and the named metrics still unassigned are cyclomatic complexity, CRAP, logical lines, parameter count, nesting depth, and function length. Which rule id measures which metric, and which stand-in warn and gate numbers ship before calibration?

## Answer

Resolved 2026-10-27 with Christopher. Why each rule has one metric and a hard stand-in: [ADR 0022](../../../docs/adr/0022-rule-ids-and-stand-in-thresholds.md). The glossary terms are **Cyclomatic complexity**, **CRAP**, **Parameter count**, **Nesting**, **Function length**, and **Logical line** in `CONTEXT.md`.

A value greater than the gate fails. The gate number itself passes. The stand-ins are the same in every language until the validation run loosens one. F03 keeps its warn. The rules assigned here have no warn level. Weights stay unset. How any gate is waived is [How a gate is waived](44-waivers.md).

F01 is cyclomatic complexity, gate 8. F02 is CRAP, gate 8, and absent coverage leaves it uncomputed. F03 stays cognitive complexity, warn above 10 and gate above 15. F04 is parameter count, gate 3, receiver excluded. F05 is nesting, gate 2. F06 is function length, gate 19, so 20 physical lines of the body span fail. F07 stays the pass-through warn at 80. F08 stays the member-chain gate at 2 hops from the receiver and 1 otherwise. F11 is logical lines, gate 19, so 20 statements fail.
