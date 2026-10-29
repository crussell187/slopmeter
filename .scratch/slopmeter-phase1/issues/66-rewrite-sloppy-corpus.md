# Rewrite the sloppy corpora so they are sloppy, not gate plus one

Type: task
Status: resolved
Blocked by: 46

## Question

Each sloppy tree's maxima sit at exactly the old gate plus one (Python F01 49, F03 63, F04 20, F06 197; Rust F06 582; C++ F06 715), so they were authored to the gates and cannot falsify them (review finding 1). After recalibration, regenerate each tree from its `seed.txt` with no knowledge of the thresholds, keep it only when every gating rule fires at the new gates by a wide margin, and re-pin. Extend the seeds so the trees also exercise the coverage gate (an untested branch) once that rule exists.

## Answer

Each `corpus/*/sloppy` tree was written again from its seed as a room-booking program. The seeds now say the uncalled function's lines are in the LCOV report and unhit. A tree is kept when the worst measurement on every gating rule is at least one and a half times that threshold and more than one past it, the pass-through forwards every ordinary parameter, and `hold` is covered at 0. The corpus tests lock that check.

The worst values are the same shape in all three languages. `choose` is F01 24 and, fully covered, F02 24. `arrange` is F03 91 and F05 7. `book` is F04 20. `describe` is F06 244 in Python and 245 in Rust and C++. `forward` is F07 100. `city` is F08 6. `record` is F11 73 in Python and 74 in Rust and C++. `hold` is the only inherited row, an S06 at 0. The Python program runs. The Rust and C++ sources typecheck. [The validation run tunes on nine pinned repos](../../../docs/adr/0013-validation-corpus.md) records the wide-margin keep.
