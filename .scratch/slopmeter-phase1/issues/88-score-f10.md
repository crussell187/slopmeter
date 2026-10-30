# Score a void function that mutates an argument as F10

Type: task
Status: resolved
Blocked by: 

## Question

Build F10 per [ADR 0036](../../../docs/adr/0036-three-warns-where-they-apply.md) in Python and C++. The Python adapter records, on a function with no `return` carrying a value, an assignment or augmented assignment whose target is a parameter's attribute or subscript, or a call of `append`, `extend`, `insert`, `pop`, `remove`, `clear`, `sort`, `reverse`, `update`, `setdefault`, or `popitem` on a parameter; `self` and `cls` are not parameters for this rule. The C++ adapter records a function whose return type is `void` and that has a non-const reference or non-const pointer parameter. `rules::evaluate` emits one weight-1 warn at the first site, the Rust adapter records nothing, the explain page says Rust is not applicable, and the Python and C++ sloppy corpora gain one such function.

## Answer

Resolved 2026-10-30. The Python adapter records, on a function with no return of a value, an assignment or augmented assignment whose target is a parameter's attribute or subscript, and a call of `append`, `extend`, `insert`, `pop`, `remove`, `clear`, `sort`, `reverse`, `update`, `setdefault`, or `popitem` on a parameter. `self` and `cls` are not parameters, and a nested function keeps its own sites. The C++ adapter records each non-const reference or non-const pointer parameter of a function whose written return type is `void`, including a trailing `void` and `T* const`. A const reference, a pointer to const, a by-value parameter, and a constructor record nothing. The Rust adapter records nothing. `rules::evaluate` emits one weight-1 warn at the first site. The measured value is the count of those sites and the threshold is 0. A waiver line for F10 fails the run. `explain F10` says Rust is not applicable. The Python and C++ sloppy corpora each gained one such function. `slopmeter --strict` on this repo still exits 0.
