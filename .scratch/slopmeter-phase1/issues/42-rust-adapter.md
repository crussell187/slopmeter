# Rust adapter and its validation run

Type: task
Status: resolved
Blocked by: 41

## Question

Implement session 15 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `adapters::rust::parse` uses `tree-sitter-rust` 0.24.2. The extension arm is the only new language match. `//` waiver lines are recorded the same way as in [Record waiver lines on the function](45-waiver-lines.md). Then the Rust corpus, on the same thresholds rule as the Python run.

## Answer

`adapters::rust::parse` records a function with a body. An inherent method is `Take::into_inner`. A trait impl method is `<Take<T> as Buf>::chunks_vectored`. A closure inside a function folds into that function. A closure bound to a const or static outside any function is its own function. `//` waiver lines directly above the touch span are recorded and do not apply the gate. The pins are `corpus/rust/pins.txt`: `serde-rs/serde` `0db9b3b504bcb0a7fa9cf001429869519afd3e9a`, `rust-lang/regex` `ad23cc29c535caf05c4050d9e4f7401d27d2d7fa`, `tokio-rs/bytes` `c08f87663a04eac3b9ab2dfa1976852d9b525a8c`. Rust gates moved to the highest value that still passes: F01 94, F03 192, F04 9, F05 9, F06 581, F08 2 hops from the receiver and 11 otherwise, F11 138. F02 stays 8. No coverage report was given, so CRAP was not computed. F03's warn stays 10 and F07 stays 80. Python stays on its calibrated gates. C++ keeps the stand-ins. `regex-capi/include/rure.h` is still refused, so a strict run of that whole checkout exits 2 until the C++ adapter. The rust sources in the pin have no gate findings. `corpus/rust/sloppy` was written from `corpus/rust/seed.txt`. With its coverage report, a strict run gates on F01–F06, F08, and F11 and warns on F07. The run is not in CI. The code-level plan is [Rust adapter](../../../docs/superpowers/plans/2026-10-27-rust-adapter.md).
