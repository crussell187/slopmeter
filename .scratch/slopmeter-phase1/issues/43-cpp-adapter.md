# C++ adapter and its validation run

Type: task
Status: resolved
Blocked by: 42

## Question

Implement session 16 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `adapters::cpp::parse` pins `tree-sitter-cpp` to `c009222808634c1014f82438d4883753516a2c24`. Both `#if` arms are scored. A literal `#if 0` is dropped. `//` waiver lines are recorded the same way as in [Record waiver lines on the function](45-waiver-lines.md). Then the C++ corpus.

## Answer

`adapters::cpp::parse` records a function with a written body. A prototype, `= default`, `= delete`, and a pure virtual are not functions. A method is `CLI::detail::join`, with the parameter types and the cv/ref qualifier beside the name. A lambda inside a function folds into that function. A lambda bound to a name outside any function is its own function. Both arms of `#if`, `#ifdef`, `#elif`, and `#else` count. A condition that is the token `0`, parentheses allowed, drops that arm. `//` waiver lines directly above the touch span are recorded and do not apply the gate. The pins are `corpus/cpp/pins.txt`: `fmtlib/fmt` `f87a75e15262614ce8aa0e6a0664df9d0823fb92`, `google/re2` `2da0056814cf180480a19f5cf811e7e1c054bf6d`, `CLIUtils/CLI11` `1764a057b760e8873eab03f4635b4aaed5303a04`. C++ gates moved to the highest value that still passes: F01 136, F03 295, F04 18, F05 8, F06 713, F08 3 hops from the receiver and 14 otherwise, F11 217. F02 stays 8. No coverage report was given, so CRAP was not computed. F03's warn stays 10 and F07 stays 80. Python and Rust stay on their calibrated gates. `corpus/cpp/sloppy` was written from `corpus/cpp/seed.txt`. With its coverage report, a strict run gates on F01–F06, F08, and F11 and warns on F07. The run is not in CI. The code-level plan is [C++ adapter](../../../docs/superpowers/plans/2026-10-27-cpp-adapter.md).
