# What a pass-through is when one extra statement dodges it

Type: grilling
Status: open
Blocked by: 

## Question

F07 requires exactly one statement (`python.rs:1027`, `rust.rs:940`), so `log(); return run(a, b, c)` or `Ok(run3(a, b))` is not a pass-through, while `return sorted(xs, key=…)` and any one-parameter function with a single call are 100% (review finding 15). `std::forward<Args>(args)...` is not recognised as forwarding (`cpp.rs:1251`). The spec's counter said body-statement count excludes logging and assignments to unused names. Decide what the pass-through shape is: which statements are ignorable, whether a wrapper call (`Ok`, `Some`, `await`) still forwards, whether the forwarded names must be ordinary parameters rather than the call's object, and whether a pack expansion forwards. Then fix all three adapters to the one definition.
