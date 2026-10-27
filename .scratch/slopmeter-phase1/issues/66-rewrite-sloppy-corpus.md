# Rewrite the sloppy corpora so they are sloppy, not gate plus one

Type: task
Status: open
Blocked by: 46

## Question

Each sloppy tree's maxima sit at exactly the old gate plus one (Python F01 49, F03 63, F04 20, F06 197; Rust F06 582; C++ F06 715), so they were authored to the gates and cannot falsify them (review finding 1). After recalibration, regenerate each tree from its `seed.txt` with no knowledge of the thresholds, keep it only when every gating rule fires at the new gates by a wide margin, and re-pin. Extend the seeds so the trees also exercise the coverage gate (an untested branch) once that rule exists.
