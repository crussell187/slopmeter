# Align the Python adapter with ADR 0003 and 0009

Type: task
Status: resolved
Blocked by: 

## Question

Three Python departures from the ADRs (review findings 14 and 17): a bare call becomes a chain hop at `src/adapters/python.rs:1218`, so `return make().a.b.c` gates at depth 4 while the Rust adapter drops the same root; loop and try `else` add +1 to CC at `:731`, `:746`, `:779` although ADR 0003 says a residual arm adds nothing (if the +1 is wanted to match radon, list it in the ADR's deltas instead); and `classify` at `:527` walks only the body, so a lambda in a parameter default or a decorator argument is unscored. Also tighten the waiver anchor so a trailing comment on a code line is not a waiver (`python.rs:261`, `rust.rs:350`).

## Answer

A bare call is a hop only when nothing is dotted off it. `return make().a.b.c` is the members `a`, `b`, and `c` in Python and in Rust. `(make()).a.b` drops `make` the same way. `send()` stays one call hop, and `obj.method().extra` keeps `method` and `extra`.

A `for`, `while`, or `try` `else` adds no decision. It still adds the hybrid cognitive increment. Radon counts that residual arm, so the cross-check records `loop_else` as slopmeter 2 against radon 3, and [Cyclomatic complexity counts decision points](../../../docs/adr/0003-cyclomatic-complexity-rules.md) lists the delta.

A lambda in a parameter default or a decorator argument is scored on the function being defined, including a nested function. The enclosing function does not also count it. The decorator's own call is not a member chain. `def defaults(a, key=lambda x: x if x and x > 1 else 0)` has both of those decisions. `@register(lambda x: x if x else 0, when=a and b)` has the lambda's decision and the `and`.

A waiver comment has to occupy its line alone. `x = 1  # slopmeter: waive ...` and `fn earlier() { 1 } // slopmeter: waive ...` do not waive the next function. The same `//` comment in C++ does not either. A comment line that is only the waiver still does.
