---
status: accepted
date: 2026-10-27
---

# A waiver excuses one gate up to a stated measurement

Every function-scope gate can fail a touched function, and `slopmeter.toml` can raise a gate for a whole language and cannot turn a rule off. The exception is a comment on that function. The tool checks the comment's shape and its number. It does not grade the prose. A person reads the reason in the diff and on the table row.

A waiver is a contiguous run of single-line comments directly above the function's touch span. The run sits above a decorator or attribute when the function has one, and above the signature otherwise. Those lines join the touch span. They add nothing to function length or to logical lines. A change to one of them touches the function. A blank line or any other comment ends the run. A comment inside a body does not waive that function. A waiver directly above a nested function waives the nested function, and that line still counts toward the enclosing function's length because it lies in the enclosing body.

Python uses `#`. Rust and C++ use `//`. After that marker and any indentation, the tokens are `slopmeter:`, `waive`, the rule id, and the number. The trimmed remainder of the line is the reason, and it has to contain text. CRAP's number may be fractional. Every other gate takes a whole number. A doc comment or a block comment is an ordinary comment. One line excuses one gate. The gates are F01, F02, F03, F04, F05, F06, F08, F11, and F15.

While the measurement is over the gate and at most the written number, the finding stays a gate in `findings` or `inherited`, keeps its weight in the slop score, and the verdict ignores it. The threshold on the finding stays the gate. The table row keeps the worst site in the function and also shows the excused number and the reason. One finding per rule, so a waived F03 gate does not also appear as the warn. A well-formed waiver on a measurement that is not over the gate, including an absent CRAP, stays attached and does not fail the run. The number is compared to the finding's measured value. For F08 that value is the depth at the worst chain.

## Considered options

- **A per-function entry in `slopmeter.toml`.** Rejected. The reason has to sit in the diff a person reviews. The config file stays a language-wide departure, and it still cannot turn a rule off.
- **A comment inside the body.** Rejected. That line is a physical line of the body, so a length waiver would trip the rule it excuses.
- **An unbounded comment.** Rejected. The function can then grow without the waiver line changing, and a later diff can hide the ceiling. The tool checks the number. The person reads the reason.
- **Drop the finding, or relabel it a warn.** Rejected. The table is the check a person reads, and the validation bar is still no gate findings on a clean repo. Warn is a lower threshold. F03's warn is a different band, and the other gates have none.
- **Report a bad line as a note.** Rejected. A note does not change the verdict. A line the tool can see and cannot accept has to fail the check.

## Consequences

- The adapter records every line in the run. It records the source line, the rule token, the number token when one is present, and the reason when text is present. It does not apply a gate. Later stages still do not read source. The Python adapter grows this record before the rule engine, in [Record waiver lines on the function](../../.scratch/slopmeter-phase1/issues/45-waiver-lines.md). Rust and C++ record the same shape when those adapters land.
- A line whose tokens begin with `slopmeter:` and `waive` and that cannot be accepted fails the check. The check is still produced and exits 1. The line stays in the run, so it does not cut off a valid waiver above it. Missing a number or a reason, a fraction on a whole-number gate, the same rule twice, and a rule that is not a gate are all rejected. A valid line for a different rule still waives that rule.
- A rejected line with no metric finding is a gate finding sited at that line. The row names the rule when the id is a real function rule. A line that names no rule is the finding `waiver`. The row shows the measurement when the function has one, and says what the line lacks. Its weight is 0, so it does not move the slop score. When the metric is already over its gate, that metric keeps the one finding and its weight, and the row says the waiver was rejected.
- A well-formed number below the measurement fails the metric finding. The site stays the worst line in the function. The row shows the measurement against the gate, and it shows the excused number and the reason.
- The finding carries the excused measurement and the reason whenever a waiver line for that rule is attached. The JSON report and the table both show them. A gate row with no waiver line stays the measurement against its threshold.
- Every gate's `explain` page carries one shared sentence: the comment shape, the number, and the reason. F07's page says a waiver line for it fails the run, because F07 is not a gate. The words are [Author the nine explain pages](../../.scratch/slopmeter-phase1/issues/27-explain-pages.md).
- A waived gate remains a gate finding. A clean validation repo still has to have none. Raising the language-wide gate in `slopmeter.toml` stays a separate departure.
