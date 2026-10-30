# Run coverage and mutation in CI and gate the pull request's touched functions

Type: task
Status: resolved
Blocked by: 80

## Question

`.github/workflows/ci.yml` runs `cargo run --quiet -- --strict` with no report, and under `--strict` every S06 and S07 row stays inherited ([ADR 0026](../../../docs/adr/0026-coverage-gate-on-a-changed-function.md), [ADR 0027](../../../docs/adr/0027-mutation-score-on-a-changed-function.md)), so this repo is never held to the coverage and mutation gates it ships. Add a pull-request job that runs `cargo llvm-cov --lcov` and `cargo mutants --in-diff` against the merge base, then `slopmeter --changed origin/main --coverage <lcov> --mutation-rust mutants.out/outcomes.json`, and fails the job on exit 1. A push to `main` has an empty diff and keeps the `--strict` run only. Record the two tool versions the job pins and the time a mutants run takes on this crate.

## Answer

The pull-request job pins cargo-llvm-cov 0.9.1 and cargo-mutants 27.1.0. It writes `lcov.info`, diffs the merge base with `origin/main` into `git.diff`, and runs `cargo mutants --copy-vcs true --in-diff git.diff`. The copy includes `.git`, because without it the unmutated baseline fails `the_repo_config_leaves_the_corpus_unscored` and cargo-mutants exits 4 before any mutant runs. Exits 2 and 3 still leave `mutants.out/outcomes.json`; any other mutants exit fails the job. The job then runs `cargo run --quiet -- --changed origin/main --coverage lcov.info --mutation-rust mutants.out/outcomes.json`, and that command's exit 1 fails the job. A push to `main` keeps the fmt, clippy, test, and `--strict` job only.

This crate lists 4117 mutants. A completed run of the 20 mutants in `src/explain.rs` took 78 seconds on this 28-core machine, baseline included (7s build + 5s test), and all 20 were caught. Those 20 ran together, so a run of all 4117 is about 4117/20 times the 66 seconds after the baseline, roughly 3.8 hours, before the copies contend for disk. The pull-request job tests the mutants in the diff.
