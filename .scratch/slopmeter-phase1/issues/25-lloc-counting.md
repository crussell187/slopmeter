# What increments a logical line

Type: grilling
Status: open
Blocked by: 19

## Question

The Phase 1 IR stores one `lloc` integer per function, filled by the adapter. [How radon, rust-code-analysis and lizard count cyclomatic complexity](04-cross-check-tools.md) shows the rulers disagree: radon applies a colon heuristic at file level and over-counts a dict literal, rust-code-analysis counts statement nodes per function, and lizard reports physical length. Which lines increment `lloc` in Python, Rust, and C++, and do blanks, comments, docstrings, braces, and attributes count?
