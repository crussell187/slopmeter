# Stop scoring test modules, fuzz harnesses and testing directories

Type: task
Status: open
Blocked by: 

## Question

ADR 0005 says `#[cfg(test)]` modules are out, but the Rust adapter has no cfg handling and `select.rs` is path-only, so 513 of 5,791 corpus functions were inline tests (review finding 2). Exclude `#[cfg(test)]` items and `#[test]` functions in the Rust adapter unless `--tests` is set; add `fuzz` and `testing` to the excluded directory segments in `src/inputs/select.rs:143-162`; add a regression test for each. Done when the 11-assert `#[test]` fixture in the review is absent from a `--strict` run and present under `--tests`.
