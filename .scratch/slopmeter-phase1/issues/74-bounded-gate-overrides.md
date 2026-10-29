# Accept a bounded count gate and note the departure

Type: task
Status: open
Blocked by: 62

## Question

Implement [A count gate may rise to twice the built-in](../../../docs/adr/0028-count-gate-twice-the-built-in.md). `slopmeter.toml` accepts `gate` on F01, F02, F03, F04, F05, F06, and F11. `warn` stays legal only on F03. F07 has no keys. F08 keeps `allow` only. S06 and S07 take neither key. A count gate may be lowered to 0, except F03, whose floor is 1. A gate may be raised to twice that language's built-in and no further. F03's warn may run from 0 to 20 and, after the merge, sits strictly below that language's gate. An omitted warn stays 10. An omitted gate stays the language's built-in, so a warn-only table does not reset the gate to 15. A number outside that range fails before scoring and the process exits 2. The error names the key.

Each value that differs from the built-in is one run-level note with no path. A value written equal to the built-in adds no note. The messages are `{language} {rule} gate is {value}, built-in {builtin}`, `{language} F03 warn is {value}, built-in {builtin}`, and `{language} F08 allow adds {names}` with the names in file order. Notes come out Python, then Rust, then C++, and within a language in the order F01, F02, F03 warn, F03 gate, F04, F05, F06, F08, F11. They precede the notes the run already emits. A departure is named even when that language contributes no file. A finding uses the value in effect. `explain` still prints the shipped threshold.
