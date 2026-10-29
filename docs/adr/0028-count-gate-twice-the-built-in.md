---
status: accepted
date: 2026-10-28
---

# A count gate may rise to twice the built-in

A project can loosen a noisy count without a release of slopmeter, and an agent can set that same number high enough that the rule never fails. The config file may set `gate` on F01, F02, F03, F04, F05, F06, and F11. `warn` stays legal only on F03. F07 has no keys. F08 keeps `allow` only. S06 and S07 take neither key. A count gate may be lowered to 0, except F03, whose floor is 1. A gate may be raised to twice that language's built-in and no further. F03's warn may run from 0 to 20, and after the merge it sits strictly below that language's gate. A number outside that range fails the run before scoring. Each value that differs from the built-in is a departure: the run names it, and the note does not fail the run.

## Considered options

- **Leave every gate but F03 as a compile-time constant.** Rejected. A repo that needs a different count would wait on a release. The profile is the table a project is allowed to change.
- **Let any rule take `warn` and `gate`, including S06 and S07.** Rejected. The count rules ship no warn band. A key on the coverage gate or the mutation gate would be the waiver those rules refuse.
- **Leave the ceiling unbounded and rely on the note.** Rejected. The agent loop gates on the exit code. A note does not fail it, so a gate of four billion turns the rule off.
- **Allow a gate only to be tightened.** Rejected. The file exists so a noisy count can move. Twice the built-in is the slack. Past that the rule is off.

## Consequences

- The ceiling is that language's shipped gate. Python's cognitive-complexity gate of 58 may rise to 116. The generic stand-in of 15 is not the ceiling. An omitted `warn` stays 10. An omitted `gate` stays the language's built-in, so a warn-only table does not reset the gate to 15.
- A value written equal to the built-in is not a departure. Coverage, include, exclude, and the mutation paths are not departures.
- One note per changed level, and one note per language for the allow-list names that were added. The note has no path. The messages are `{language} {rule} gate is {value}, built-in {builtin}`, `{language} F03 warn is {value}, built-in {builtin}`, and `{language} F08 allow adds {names}` with the names in file order. Notes come out in language order, Python then Rust then C++, and within a language in the order F01, F02, F03 warn, F03 gate, F04, F05, F06, F08, F11. They precede the notes the run already emits. A departure is named even when that language contributes no file.
- A finding uses the value in effect. `explain` still prints the shipped threshold, because it does not read the repo. Weights stay fixed.
- [The config file records departures from the built-in profiles](0018-config-records-departures.md) gains this ceiling and these notes. [Which metric each rule id measures](0022-rule-ids-and-stand-in-thresholds.md) still names the built-in numbers.
