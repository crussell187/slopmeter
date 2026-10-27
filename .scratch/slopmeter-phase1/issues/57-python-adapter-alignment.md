# Align the Python adapter with ADR 0003 and 0009

Type: task
Status: open
Blocked by: 

## Question

Three Python departures from the ADRs (review findings 14 and 17): a bare call becomes a chain hop at `src/adapters/python.rs:1218`, so `return make().a.b.c` gates at depth 4 while the Rust adapter drops the same root; loop and try `else` add +1 to CC at `:731`, `:746`, `:779` although ADR 0003 says a residual arm adds nothing (if the +1 is wanted to match radon, list it in the ADR's deltas instead); and `classify` at `:527` walks only the body, so a lambda in a parameter default or a decorator argument is unscored. Also tighten the waiver anchor so a trailing comment on a code line is not a waiver (`python.rs:261`, `rust.rs:350`).
