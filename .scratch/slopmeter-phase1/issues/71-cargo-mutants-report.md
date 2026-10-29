# Read a cargo-mutants report into S07

Type: task
Status: open
Blocked by: 61

## Question

Implement [A changed function passes when every valid mutant is detected](../../../docs/adr/0027-mutation-score-on-a-changed-function.md) for Rust, including the shared rule. `[mutation] rust` and `--mutation-rust` name `mutants.out/outcomes.json`. Caught and timeout are detected, missed is undetected, and unviable is left out. Join on the file and `span.start.line` against the body span. S07 weighs 10, gates at 1, and has no warn. The explain page is `slopmeter explain S07`. A waiver line that names S07 fails the run. A missing report fails a touched function on the default run and names the path. A report that cannot be parsed exits 2. `--strict` and a run outside git keep a shown row inherited and add no row when the report is missing. A file absent from the report fails each touched function in it. A matched span with no valid mutant stays off the rule. The finding sits on the first undetected mutant, or on the function line when no score was produced. The cell reads `0.5 < 1`, or `absent < 1`.
