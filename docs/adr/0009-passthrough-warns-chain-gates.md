---
status: accepted
date: 2026-10-26
---

# A pass-through warns, and a member chain gates

Phase 1 has no symbol resolution, so it cannot count how many types implement an interface. F07 and F08 are judged from the function's own syntax, in Python, Rust, and C++ alike. F07 matches a pass-through that hands at least 80% of its ordinary parameters to one call. That finding is a warn: it is reported and scored, and it does not fail the run. F08 gates a member chain deeper than two when the chain starts at the receiver, or deeper than one otherwise. A listed stdlib member name adds no hop, and a trailing run of calls that ends in `build` adds none either.

A pass-through is that one call. A logging statement, including one that names a parameter, and an assignment to a name that is never read, may sit beside it. `return`, `await`, and `?` unwrap. `Ok`, `Some`, and `Err` peel when they wrap that call in a single argument. An ordinary parameter is handed off as an argument, as the call's object, or as a pack expansion of that hand-off. A string, a number, a boolean, or null may sit beside those arguments. Any other argument means the function added something, so it is not a pass-through. An alias is not followed: `tmp = a; return f(tmp)` is not a pass-through.

## Considered options

- **Count implementations inside the scored set and exempt an interface with two or more.** Rejected. An implementation in a dependency is invisible, a Python base imported from another module is only a name, and a C++ virtual is not tied to its overrides. A wrong count would exempt the one-implementation pass-through the rule exists to catch.
- **Keep a body of exactly one statement.** Rejected. `log(); return run(a, b, c)` and `Ok(run(a, b))` are the same hand-off with a line added, and that line was enough to dodge the warn.
- **Count an ordinary parameter only when it is an argument.** Rejected. `repo.save(user)` is the hand-off this rule exists for, and `repo` is what carries that function to 80%.
- **Exempt a trait impl, an `override`, or `@abstractmethod`.** Rejected. The mark means the signature is required. The body can still be a pure forward.
- **Gate F07.** Rejected. Idiomatic delegation would fail clean repos, and raising the 80% line does not remove those functions.
- **Warn on F08, or leave it uncomputed.** Rejected. The depth limits and the allow-list are the noise control. The validation run can loosen the depth if clean repos fail on it.

## Consequences

- The function-scope gates are F01–F06, F08, and F11. F07 warns in every language, and Phase 1 has no gate level for it.
- F08 has no softer warn level in Phase 1. Its depth is a default the validation run can move. That run cannot give F07 a gate without the missing exemption.
- A project adds allow-list names under `allow`. The ratio and the two depths stay in the shipped profile. [The config file records departures](0018-config-records-departures.md).
