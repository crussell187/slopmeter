# Score a null literal as F12

Type: task
Status: open
Blocked by: 

## Question

Build F12 per [ADR 0036](../../../docs/adr/0036-three-warns-where-they-apply.md) in C++ only. The C++ adapter records a `return` whose expression is `nullptr` or `NULL`, and a call argument that is `nullptr` or `NULL`; a comparison against null is not a site. `rules::evaluate` emits one weight-1 warn at the first site, the Python and Rust adapters record nothing, the explain page says those languages are not applicable, and the C++ sloppy corpus gains one function that returns `nullptr`.
