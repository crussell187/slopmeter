# What a Phase 1 finding carries

Type: grilling
Status: resolved
Blocked by: 

## Question

Each finding names rule, symbol, file:line, measured value, threshold, weight, a one-sentence `principle` with its source, and a `fix_shape` template filled with the numbers. The spec schedules `anti_fix` text for Phase 3 but also says every Tier A rule ships with its counter. Does Phase 1 include `anti_fix` on findings, in `explain` only, or not at all? Does `fix_shape` compute the projected number ("extract the branch block at lines 40–62 and CC drops to ≈ 6") in Phase 1, or stay a fixed template?

## Answer

Resolved 2026-10-26 with Christopher. Why the finding reports the worst site and does not plan the edit: [ADR 0010](../../../docs/adr/0010-finding-payload.md). The glossary terms are **Finding**, **Fix shape**, **Anti-fix**, and **Gate** in `CONTEXT.md`.

A finding is the worst violation of one rule at one function. It carries the rule, the function's identity, the line of that site, the measured value, the one threshold it is over, the level `gate` or `warn`, the rule's weight, a one-sentence principle with its source, the fix shape, and one anti-fix sentence. The sentence is the same for every finding of that rule, on Tier A and on Tier B. It names the cheapest edit that would make the number look better and states that the finding remains. The longer anti-fix list stays in `explain`, on [Text table and explain output format](23-text-and-explain-output.md).

The symbol is the file of the body plus the qualified name, with C++ parameter types. A function with two sites over the limit produces one finding. The measured value is the worse site. Equal values keep the earlier line. The next run reports the other site if it is still over. F07 is located at the forwarding statement. F08 is located at the line where the worst chain starts, and the fix shape names that chain. Complexity, length, and parameter count are located at the `def` or `fn` line.

The fix shape names the refactoring and fills in the measured value, that threshold, and the function's location. It does not name an interior span and does not predict the number after the edit.

A rule with a warn level and a gate level puts one of those numbers on the finding, the level the value is over. Cyclomatic complexity at 18 carries threshold 15 and level `gate`. At 12 it carries threshold 10 and level `warn`. F07 carries level `warn` and threshold 80, and Phase 1 has no gate level for it. F08 carries level `gate` and the depth limit it crossed, and Phase 1 has no softer warn level. An inherited finding keeps that same level and threshold. Which list it sits in, so that it does not fail the run, stays on [Define the JSON report schema](21-report-schema.md).

The finding carries the weight the rule assigns. The weight numbers themselves wait for the validation run.
