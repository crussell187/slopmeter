# Stop scoring test modules, fuzz harnesses and testing directories

Type: task
Status: resolved
Blocked by: 

## Question

ADR 0005 says `#[cfg(test)]` modules are out, but the Rust adapter has no cfg handling and `select.rs` is path-only, so 513 of 5,791 corpus functions were inline tests (review finding 2). Exclude `#[cfg(test)]` items and `#[test]` functions in the Rust adapter unless `--tests` is set; add `fuzz` and `testing` to the excluded directory segments in `src/inputs/select.rs:143-162`; add a regression test for each. Done when the 11-assert `#[test]` fixture in the review is absent from a `--strict` run and present under `--tests`.

## Answer

`inputs::select` treats a `fuzz` or `testing` directory the same way as `test` and `tests`. Those files stay out until `--tests`. A file named `testing.rs` stays in, and `target/` stays out either way. `adapters::rust::parse` drops a `#[test]` function and a `#[cfg(test)]` item, helpers inside that module included. `#[cfg(not(test))]` stays. `--tests` passes the flag through, and `parse_with` records those functions. An 11-assert `#[test]` function in `src/lib.rs` is absent from a `--strict` run and present under `--tests`, where its parameter count gates on F04. The code-level plan is [Scored-set leaks](../../../docs/superpowers/plans/2026-10-27-scored-set-leaks.md).
