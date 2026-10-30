# Run coverage and mutation in CI and gate the pull request's touched functions

Type: task
Status: open
Blocked by: 80

## Question

`.github/workflows/ci.yml` runs `cargo run --quiet -- --strict` with no report, and under `--strict` every S06 and S07 row stays inherited ([ADR 0026](../../../docs/adr/0026-coverage-gate-on-a-changed-function.md), [ADR 0027](../../../docs/adr/0027-mutation-score-on-a-changed-function.md)), so this repo is never held to the coverage and mutation gates it ships. Add a pull-request job that runs `cargo llvm-cov --lcov` and `cargo mutants --in-diff` against the merge base, then `slopmeter --changed origin/main --coverage <lcov> --mutation-rust mutants.out/outcomes.json`, and fails the job on exit 1. A push to `main` has an empty diff and keeps the `--strict` run only. Record the two tool versions the job pins and the time a mutants run takes on this crate.
