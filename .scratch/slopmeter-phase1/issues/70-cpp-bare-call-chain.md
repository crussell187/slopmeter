# Drop a C++ bare call that only starts a member chain

Type: task
Status: resolved
Blocked by:

## Question

Python and Rust no longer count a bare call as a hop when member access continues it, so `make().a.b.c` is `a`, `b`, and `c`. C++ `peel` still records the call name, so the same shape is one hop deeper. Drop that root the same way. A bare call that is the whole chain stays one hop.

## Answer

Resolved 2026-10-29. `adapters::cpp` drops a bare call that only starts a member chain. `make().a.b.c`, `(make()).a.b`, and `make()->a.b` are the members after the call. `obj.method().extra` keeps `method` and `extra`, and `send()` stays one call hop.
