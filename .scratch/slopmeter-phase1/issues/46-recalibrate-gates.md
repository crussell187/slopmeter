# Recalibrate the gates by percentile on a production-only set

Type: grilling
Status: open
Blocked by: 47, 50, 56, 57, 67, 70, 75

## Question

The shipped gates equal the single worst function in three clean repos per language (CC 48/94/136, length 196/581/713), so no existing function fails, and the outliers that set them were test, fuzz and generated code ([the Phase 1 review](../../../docs/review/2026-10-27-phase1-review.md), finding 1). Once the scored set and the measurement bugs are fixed, how are gates set? The audit proposes a stated percentile of the production-only corpus (p95 for CC, cognitive, logical lines and length; p97.5 for params and nesting), rounded, with the percentile recorded in ADR 0022, and these defaults with the evidence: CC 12; cognitive warn 10 / gate 15; params 6 Python, 4 Rust, 5 C++; nesting 4; length 60; logical lines 25 Python and Rust, 30 C++; CRAP 8; chains 2 from the receiver and 1 otherwise. Decide the method, the numbers, whether Python's parameter count should ignore parameters with defaults, and what the waiver policy is for a clean-repo outlier that fails at the new gate (a generated table, a 28-arm printer match). Re-pin `corpus/*/pins.txt` and re-run the nine repos; the per-function distributions are in the review's audit artifacts.
