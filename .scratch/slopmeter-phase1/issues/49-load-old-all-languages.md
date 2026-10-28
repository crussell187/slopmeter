# Make deletions in Rust and C++ files touch only the functions they hit

Type: task
Status: resolved
Blocked by: 

## Question

`load_old` in `src/scope.rs:198-202` parses the old side only for Python (a stale comment says the other adapters are later sessions), so a deletion anywhere in a Rust or C++ file touches every function in it, and the test at `src/scope.rs:1340` pins that as intended (review finding 8). Call all three adapters, invert that test, and add a fixture where deleting one line in one Rust function gates only that function.

## Answer

`load_old` parses the pre-change bytes with `adapters::parse` for Python, Rust, and C++. A deleted line gates the function whose old span contains it, matched by name, parameter types, and qualifier. Deleting one line inside `touched` leaves `other` inherited in a Rust file and in a C++ file. A `git show` that fails still touches the whole file. The code-level plan is [Load-old all languages](../../../docs/superpowers/plans/2026-10-27-load-old-all-languages.md).
