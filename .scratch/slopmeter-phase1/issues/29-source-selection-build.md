# Select the scored files

Type: task
Status: resolved
Blocked by: 28

## Question

Implement session 2 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `inputs::select` returns the scored set. A fixture tree keeps `src/a.py` and `build.rs`, and drops tests, `target/`, and protobuf names until `--tests`.

## Answer

`inputs::select(root, SelectOptions { tests })` returns repo-relative paths and a language, sorted. The default fixture keeps `build.rs` and `src/a.py`. `--tests` also keeps `tests/test_a.py`. Denylisted directories, protobuf names, and unclaimed extensions stay out. A `.gitignore` applies only when the root contains `.git`. Include and exclude globs stay with the config loader. A `#[cfg(test)]` item stays with the Rust adapter, because it is inside a file. The code-level plan is [source selection](../../../docs/superpowers/plans/2026-10-27-source-selection.md).
