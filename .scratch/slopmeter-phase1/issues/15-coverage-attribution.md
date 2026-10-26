# Attributing coverage records to IR functions, per language

Type: grilling
Status: open
Blocked by: 02, 06

## Question

Given what the coverage research found about FN/FNDA/DA records from coverage.py, cargo-llvm-cov and gcov, and given the function-identity decision: how does slopmeter attribute coverage to each function? By matching FN names (needs demangling for C++, closure naming for Rust), by intersecting DA line records with the function's line range, or both with a precedence? What coverage fraction feeds CRAP (lines, instructions, branches)? What happens for a function with no coverage data (spec: N/A, warn-only) versus a file missing from the report entirely, and does the tool optionally run the test command first as crap4java does?
