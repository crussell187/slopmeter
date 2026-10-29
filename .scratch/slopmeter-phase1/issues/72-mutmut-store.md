# Read a mutmut store into S07

Type: task
Status: resolved
Blocked by: 71

## Question

Implement the Python reader in [A changed function passes when every valid mutant is detected](../../../docs/adr/0027-mutation-score-on-a-changed-function.md). `[mutation] python` and `--mutation-python` name the `mutants/` directory. Read `mutants/<file>.meta`, one result per mutant, and ignore `export-cicd-stats`. Killed and timeout are detected, survived and no-tests are undetected, and a type-check rejection is left out. Recover the start line from that store. A skipped, suspicious, interrupted, or not-checked mutant is unfinished, and so is a mutant whose line cannot be recovered. An unfinished mutant fails a touched function in that file. The score is not taken from the finished subset.

## Answer

Resolved 2026-10-29. `inputs::score_python` reads mutmut's `mutants/<file>.meta`. Killed (`1`, `3`) and timeout (`24`, `36`, `152`, `255`, `-24`) are detected, survived (`0`) and no tests (`5`, `33`) are undetected, and a type-check rejection (`37`) is left out. `mutmut-cicd-stats.json` is ignored. The start line is the first line that differs between `__mutmut_orig` and the mutant, found again in the source file; a `.spans` index supplies those copies when mutmut wrote one. A skipped, suspicious, interrupted, or not-checked mutant is unfinished, and so is a mutant whose line cannot be recovered. That failure covers every function in the file, and the score is not taken from the finished subset. `[mutation] python` and `--mutation-python` name the directory. Files inside that directory are not scored. An unset path, or a path missing on disk, fails each touched Python function on the default run and adds no row under `--strict` or outside git. A store that cannot be parsed exits 2.
