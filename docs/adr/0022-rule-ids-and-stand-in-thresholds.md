---
status: accepted
date: 2026-10-27
---

# Each function rule has one metric and a hard stand-in

F03, F07, and F08 already had metrics. The remaining function rules are cyclomatic complexity, CRAP, parameter count, nesting, function length, and logical lines. The design spec's defaults (CC > 10, 40 logical lines, 4 parameters) were for typed class-based code and were not measured here. crap4java publishes no cyclomatic-complexity threshold. Its only failing line is CRAP greater than 8.0. The stand-ins below ship in every language. The validation run may loosen one. It may not switch a gate off.

A value greater than the gate fails. The gate number itself passes. F03 keeps its warn. The other rules decided here have no warn level.

## Considered options

- **Ship the spec defaults, with a warn band under each gate.** Rejected. A CRAP score passes or it fails, and crap4java has one line. A warn band under it reports a score the run does not fail.
- **Use lizard's flag defaults.** Rejected. A parameter ceiling of 100, a nesting flag of 7, and a length flag of 1000 are report ceilings. They count nesting differently, and they are not quality lines.
- **Give function length its own comment waiver.** Rejected. Every gate would need the same answer. That decision is [How a gate is waived](../../.scratch/slopmeter-phase1/issues/44-waivers.md).

## Consequences

- F01 is cyclomatic complexity. The gate is 8. Complexity 8 passes and 9 fails.
- F02 is CRAP. The gate is 8. A score of 8 passes and a score above 8 fails. Absent coverage leaves CRAP uncomputed, and F02 emits nothing.
- F03 stays cognitive complexity, warn above 10 and gate above 15.
- F04 is parameter count. The gate is 3. The receiver is excluded. Three ordinary parameters pass and four fail.
- F05 is nesting, the deepest decision. The gate is 2. Nesting 2 passes and 3 fails.
- F06 is function length, the physical lines of the body span. The gate is 19, so 19 lines pass and 20 fail. A decorator or attribute is outside the span.
- F07 stays the pass-through warn at 80, with no gate.
- F08 stays the member-chain gate, 2 hops from the receiver and 1 otherwise, with no warn.
- F11 is logical lines. The gate is 19, so 19 statements pass and 20 fail.
- A `warn` key is legal only on a rule that ships one. Today that is F03. F07's ratio and F08's depths stay as already decided, and F08's only config key is `allow`.
- These numbers are the built-in profile. Per-language calibration waits on the validation run. Weights stay unset.
- The rule engine is blocked on the waiver ticket, so no gate ships with a private escape hatch.
