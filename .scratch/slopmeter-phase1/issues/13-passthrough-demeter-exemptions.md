# Pass-through and Demeter detection without symbol resolution

Type: grilling
Status: resolved
Blocked by: 06

## Question

F07 flags a single-statement body forwarding ≥ 80% of its params to one callee; the spec's risk section says it must exempt classes implementing an interface with ≥ 2 implementations, which needs symbol resolution Phase 1 does not have. F08 flags member-access chains deeper than the threshold, excluding fluent builders and stdlib collections via an allow-list. For Python, Rust and C++: which exemptions can Phase 1 honour purely syntactically (trait impls in Rust, `@abstractmethod` overrides in Python, `override` in C++), what goes in the default allow-lists, and do F07/F08 ship as gates, warns, or off for any language in Phase 1 given the false-positive risk?

## Answer

Resolved 2026-10-26 with Christopher. Why a pass-through warns and a member chain gates: [ADR 0009](../../../docs/adr/0009-passthrough-warns-chain-gates.md). The glossary terms are **Pass-through** and **Member chain** in `CONTEXT.md`.

Both rules are judged from the function's own syntax, in Python, Rust, and C++ alike. The spec's "interface with at least two implementations" exemption is dropped. A trait impl, an `override` / `final`, or `@abstractmethod` does not exempt a function. `@abstractmethod` never matches F07: its body is `...` or `pass`, not a forwarding call.

F07 is a warn. It is in the report and in the score, and Phase 1 has no gate level for it. A match is a body of exactly one statement whose expression unwraps, through `return`, `co_return`, `await` / `.await`, and Rust `?`, to a single call. A Rust tail expression counts as that statement. A Python docstring is ignored. The arguments of that call, and its receiver, contain no other call, except one identity wrapper.

The receiver is left out of the ratio. The receiver is the Rust `self` parameter, the implicit object of a C++ member function, a C++23 explicit object parameter when the grammar parses one, and the first parameter of a Python function defined directly in a class. `@staticmethod` has no receiver. `@classmethod` keeps its first parameter as the receiver. A nested function is not a method. A function with no ordinary parameters stays unmatched. Parameters with defaults, and variadic parameters, are ordinary.

A parameter is forwarded when that one call receives it unchanged, as an argument or as the call's receiver. Argument order does not matter. A keyword argument forwards its value. Extra literals do not change the ratio. The hand-off forms are the bare parameter, parentheses included; one borrow or dereference (`&`, `&mut`, `*`); Python `*` or `**` unpacking of that parameter; and one identity wrapper. The wrappers are Rust `clone`, `to_owned`, and `into` with no arguments, and C++ `move` and `forward`, qualified or not. A field access or a cast does not forward the parameter. At 80% or more of the ordinary parameters, the function warns.

F08 is a gate. Depth is the number of `.` or `->` hops in one expression. A call, an index, and parentheses add no hop and do not break the chain. `::` is a path, not a hop. The chain is receiver-rooted only when it starts with the receiver or with `this`. Implicit `this` in C++ is invisible, so `repo_.save(user)` is depth 1. A receiver-rooted chain may be two deep. Any other chain may be one deep. Past that, the finding fails the run. Phase 1 has no softer warn level.

A listed member name adds no hop. The list is per language and matches the member identifier. A trailing run of calls that ends in `build` adds none either. Hops before that run still count. A project adds names under `allow`. The file shape is [Shape of slopmeter.toml](22-config-shape.md).

Python: `get`, `keys`, `values`, `items`, `append`, `extend`, `add`, `update`, `pop`, `split`, `rsplit`, `strip`, `lstrip`, `rstrip`, `join`, `lower`, `upper`, `startswith`, `endswith`, `encode`, `decode`, `format`, `copy`.

Rust: `unwrap`, `expect`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `as_ref`, `as_mut`, `as_deref`, `as_deref_mut`, `ok`, `err`, `cloned`, `copied`, `iter`, `iter_mut`, `into_iter`, `map`, `filter`, `filter_map`, `flat_map`, `flatten`, `enumerate`, `zip`, `take`, `skip`, `chain`, `collect`, `rev`, `inspect`, `clone`, `to_owned`, `to_string`, `to_vec`, `as_str`, `as_bytes`, `into`, `get`, `get_mut`, `first`, `last`, `front`, `back`, `entry`, `keys`, `values`, `lock`, `read`, `write`, `borrow`, `borrow_mut`.

C++: `begin`, `end`, `cbegin`, `cend`, `rbegin`, `rend`, `crbegin`, `crend`, `size`, `empty`, `at`, `front`, `back`, `data`, `find`, `c_str`, `value`, `value_or`, `has_value`.

The validation run can loosen F08's depth if clean repos fail on it. It cannot give F07 a gate without the missing exemption.
