# Score a null literal as F12

Type: task
Status: resolved
Blocked by: 

## Question

Build F12 per [ADR 0036](../../../docs/adr/0036-three-warns-where-they-apply.md) in C++ only. The C++ adapter records a `return` whose expression is `nullptr` or `NULL`, and a call argument that is `nullptr` or `NULL`; a comparison against null is not a site. `rules::evaluate` emits one weight-1 warn at the first site, the Python and Rust adapters record nothing, the explain page says those languages are not applicable, and the C++ sloppy corpus gains one function that returns `nullptr`.

## Answer

Resolved 2026-10-30. The C++ adapter records a `return` whose expression is `nullptr` or `NULL`, including one wrapped in parentheses, and each call argument that is `nullptr` or `NULL`. A comparison against null and an initializer that stores null are not sites. A nested method keeps its own sites, and a lambda folded into its function counts on that function. The Python and Rust adapters record nothing. `rules::evaluate` emits one weight-1 warn at the first site. The measured value is the count of those sites and the threshold is 0. A waiver line for F12 fails the run. `explain F12` says Python and Rust are not applicable. The C++ sloppy corpus gained one function that returns `nullptr`. `slopmeter --strict` on this repo still exits 0.
