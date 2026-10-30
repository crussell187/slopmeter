# Score a void function that mutates an argument as F10

Type: task
Status: open
Blocked by: 

## Question

Build F10 per [ADR 0036](../../../docs/adr/0036-three-warns-where-they-apply.md) in Python and C++. The Python adapter records, on a function with no `return` carrying a value, an assignment or augmented assignment whose target is a parameter's attribute or subscript, or a call of `append`, `extend`, `insert`, `pop`, `remove`, `clear`, `sort`, `reverse`, `update`, `setdefault`, or `popitem` on a parameter; `self` and `cls` are not parameters for this rule. The C++ adapter records a function whose return type is `void` and that has a non-const reference or non-const pointer parameter. `rules::evaluate` emits one weight-1 warn at the first site, the Rust adapter records nothing, the explain page says Rust is not applicable, and the Python and C++ sloppy corpora gain one such function.
