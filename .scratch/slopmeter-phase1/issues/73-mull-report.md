# Read a mull report into S07

Type: task
Status: resolved
Blocked by: 71

## Question

Implement the C++ reader in [A changed function passes when every valid mutant is detected](../../../docs/adr/0027-mutation-score-on-a-changed-function.md). `[mutation] cpp` and `--mutation-cpp` name mull's mutation-testing-elements JSON. Slopmeter does not read `compile_commands.json`. Killed and timeout are detected, survived and no-coverage are undetected, and a compile error or a runtime error is left out. Join on the file and `location.start.line` against the body span. A pending or ignored mutant is unfinished, and an unfinished mutant fails a touched function in that file.

## Answer

Resolved 2026-10-29. `inputs::score_cpp` reads mull's mutation-testing-elements JSON. Killed and timeout are detected, survived and no coverage are undetected, and a compile error or a runtime error is left out. The join is the file and `location.start.line` against the body span. A pending or ignored mutant is unfinished, and so is a mutant whose start line is missing. That failure covers every function in the file. `[mutation] cpp` and `--mutation-cpp` name the report. Slopmeter does not read `compile_commands.json`. An unset path, or a path missing on disk, fails each touched C++ function on the default run and adds no row under `--strict` or outside git. A report that cannot be parsed exits 2.
