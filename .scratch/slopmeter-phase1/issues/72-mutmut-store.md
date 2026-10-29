# Read a mutmut store into S07

Type: task
Status: open
Blocked by: 71

## Question

Implement the Python reader in [A changed function passes when every valid mutant is detected](../../../docs/adr/0027-mutation-score-on-a-changed-function.md). `[mutation] python` and `--mutation-python` name the `mutants/` directory. Read `mutants/<file>.meta`, one result per mutant, and ignore `export-cicd-stats`. Killed and timeout are detected, survived and no-tests are undetected, and a type-check rejection is left out. Recover the start line from that store. A skipped, suspicious, interrupted, or not-checked mutant is unfinished, and so is a mutant whose line cannot be recovered. An unfinished mutant fails a touched function in that file. The score is not taken from the finished subset.
