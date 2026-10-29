# Whether the self-check joins CI while the crate is over its gates

Type: grilling
Status: open
Blocked by:

## Question

`slopmeter --strict` on this repo, with `corpus/` excluded, exits 1. Measured 2026-10-29: 364 gate findings, 357 in `src/` and 7 in `prototype/ir-schema/`, on F01 (49), F03 (92), F04 (47), F05 (5), F06 (19), F08 (117), and F11 (35). The worst are `Classifier::walk` in the C++ adapter at cyclomatic 59 against 10, and `git` in `src/scope.rs` at a builder chain 11 deep against 1. [One crate, a public repo, and notes on main](../../../docs/adr/0014-one-crate-public-repo.md) says that command joins CI once the thresholds exist. The thresholds exist and the command fails. Does the self-check wait until this crate is under the gates, join now and fail the build, or does this repo record a departure that lets the check pass?
