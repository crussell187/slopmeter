# What a pass-through is when one extra statement dodges it

Type: grilling
Status: resolved
Blocked by: 

## Question

F07 requires exactly one statement (`python.rs:1027`, `rust.rs:940`), so `log(); return run(a, b, c)` or `Ok(run3(a, b))` is not a pass-through, while `return sorted(xs, key=…)` and any one-parameter function with a single call are 100% (review finding 15). `std::forward<Args>(args)...` is not recognised as forwarding (`cpp.rs:1251`). The spec's counter said body-statement count excludes logging and assignments to unused names. Decide what the pass-through shape is: which statements are ignorable, whether a wrapper call (`Ok`, `Some`, `await`) still forwards, whether the forwarded names must be ordinary parameters rather than the call's object, and whether a pack expansion forwards. Then fix all three adapters to the one definition.

## Comments

- 2026-10-28: A pass-through body is one forwarding statement. A logging statement still counts as noise when it mentions a parameter; the callee or macro's last identifier is `log`, `logger`, `debug`, `info`, `warn`, `warning`, `error`, `trace`, `print`, `println`, `eprintln`, or `dbg`. An assignment whose target never appears again in the body is noise. A branch, a loop, an assert, a second ordinary call, or an assignment that is read later is not. Alias-following is not part of this.
- 2026-10-28: Peel repeated layers of `Ok`, `Some`, and `Err` when that call has a single argument and the argument is the forwarding call. `Box::new` and any other unary name stay outside the peel. `return`, `co_return`, `await`, `.await`, and Rust `?` stay as already unwrapped.
- 2026-10-28: An ordinary parameter is forwarded when the call receives it as an argument or as the call's object. `repo.save(user)` forwards both. `xs.lower()` forwards `xs`.
- 2026-10-28: An argument that is neither a hand-off of an ordinary parameter nor a literal disqualifies the call. `sorted(xs, key=str.lower)` and `sorted(xs, key=lambda x: x)` are not pass-throughs. `sorted(xs, reverse=True)`, `request("get", url, params=params)`, `len(xs)`, and `xs.lower()` still are.
- 2026-10-28: A pack expansion forwards the pack when its pattern is a hand-off. `send(std::forward<Args>(args)...)` and `send(args...)` forward `args`. `send(foo(args)...)` does not. A fold expression does not. Python `*args` and `**kwargs` already forward.

## Answer

Resolved 2026-10-28 with Christopher. The shape is an amendment to [ADR 0009](../../../docs/adr/0009-passthrough-warns-chain-gates.md). The glossary term is **Pass-through** in `CONTEXT.md`.

A pass-through is one call that hands ordinary parameters off unchanged, as an argument, as the call's object, or as a pack expansion of that hand-off. At 80% or more, it warns. Beside that call, a logging statement still counts when it names a parameter. The callee or macro's last identifier is `log`, `logger`, `debug`, `info`, `warn`, `warning`, `error`, `trace`, `print`, `println`, `eprintln`, or `dbg`. An assignment whose target never appears again in the body is ignored. A string, a number, a boolean, or null may sit in the argument list. Any other argument disqualifies the call. Repeated layers of `Ok`, `Some`, and `Err` peel when the wrapper has a single argument. `return`, `co_return`, `await`, `.await`, and Rust `?` still unwrap. `Box::new` does not peel. `tmp = a; return f(tmp)` is not a pass-through.

`adapters::python::parse`, `adapters::rust::parse`, and `adapters::cpp::parse` record that call. `send(std::forward<Args>(args)...)` and `send(args...)` forward `args`. `sorted(xs, key=str.lower)` does not.
