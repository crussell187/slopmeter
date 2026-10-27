# Make deletions in Rust and C++ files touch only the functions they hit

Type: task
Status: open
Blocked by: 

## Question

`load_old` in `src/scope.rs:198-202` parses the old side only for Python (a stale comment says the other adapters are later sessions), so a deletion anywhere in a Rust or C++ file touches every function in it, and the test at `src/scope.rs:1340` pins that as intended (review finding 8). Call all three adapters, invert that test, and add a fixture where deleting one line in one Rust function gates only that function.
