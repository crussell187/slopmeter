# Cognitive complexity increments

Type: grilling
Status: open
Blocked by: 06, 07

## Question

F03 is not cyclomatic complexity. [SonarSource cognitive complexity rules mapped to Python, Rust, C++](03-cognitive-complexity.md) says to implement Sonar white paper v1.7 Appendix B, and leaves these disagreements open: recursion (unimplemented in every analyzer), lambda and nested-function nesting (sonar-python does not raise nesting for a lambda; Rust and C++ do), and `match`/`switch` (sonar-python adds nothing; Rust and C++ add 1 and raise nesting). Cyclomatic complexity already folds a lambda's branches into the enclosing function and counts each specified match label. Which of those Sonar increments does F03 use, where does it follow the cyclomatic rules instead, and which labelled jumps count? Threshold numbers stay in the calibration fog. Clippy's `cognitive_complexity` lint is a different metric and is not the source.
