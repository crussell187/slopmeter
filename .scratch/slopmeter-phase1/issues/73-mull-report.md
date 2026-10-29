# Read a mull report into S07

Type: task
Status: open
Blocked by: 71

## Question

Implement the C++ reader in [A changed function passes when every valid mutant is detected](../../../docs/adr/0027-mutation-score-on-a-changed-function.md). `[mutation] cpp` and `--mutation-cpp` name mull's mutation-testing-elements JSON. Slopmeter does not read `compile_commands.json`. Killed and timeout are detected, survived and no-coverage are undetected, and a compile error or a runtime error is left out. Join on the file and `location.start.line` against the body span. A pending or ignored mutant is unfinished, and an unfinished mutant fails a touched function in that file.
