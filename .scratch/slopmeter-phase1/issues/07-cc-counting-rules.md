# Cyclomatic complexity counting rules, per language

Type: grilling
Status: open
Blocked by: 04

## Question

Starting from crap4java's rules (base 1; +1 per if, loop, catch, ?:, switch label, &&, ||) and what the cross-check research found radon, rust-code-analysis and lizard do, which constructs increment CC in slopmeter for Python (elif, comprehension `if`/`for`, `and`/`or`, `with`, `assert`, ternary, `match` cases, except clauses), Rust (match arms, `if let`/`while let`, `?`, `&&`/`||`, let-else, closures), and C++ (switch cases incl. default, `?:`, `&&`/`||`, catch, range-for, goto, preprocessor-conditional branches)? Do we aim for parity with one existing tool per language so the validation cross-check can demand equal numbers, or document our own rules and accept a bounded delta?
