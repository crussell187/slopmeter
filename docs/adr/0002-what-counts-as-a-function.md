---
status: accepted
date: 2026-10-26
---

# A function is a named body, and a lambda folds into it

Function-scope rules need one definition of what they measure, and the obvious one — every lambda is its own function, as Sonar does — is the cheapest way to clear a sprawling function. A function is a named definition with a body the programmer wrote, including one nested inside another. A lambda or closure inside a function belongs to that function, so its branches and lines count there. A lambda bound to a name outside any function is a function, because otherwise it would be invisible. Declarations, `= default`, `= delete`, decorators, and attributes sit outside. A generic or template counts once. Identity is the path of the file that holds the body, plus one name qualified inside that file, plus the C++ parameter types, plus the C++ cv and ref qualifier. Python is dotted (`get`, `Response.json`, `outer.inner`). An inherent Rust method is `Take::into_inner`. A trait method is `<Take<T> as Buf>::chunks_vectored`, so it does not collide with an inherent method. C++ is `CLI::detail::join`, with the parameter types and the qualifier beside the name, and without the return type or the parameter names. The body span's start line is only a tie-break when that whole identity occurs twice in one file. A hash of the body is not an identity: an edit would make an inherited finding miss the same function.

## Considered options

- **Score every lambda on its own.** Matches Sonar, which does not add a lambda's internals to the enclosing method. Rejected: moving the mess into a lambda would drop the parent under the threshold.
- **Identity is the file and the start line, or a hash of the body.** Rejected: a blank line, or any edit, would make the function look new.

## Consequences

- F03 will not match Sonar's number on a function that contains a lambda. Increment rules stay on [Cyclomatic complexity counting rules, per language](../../.scratch/slopmeter-phase1/issues/07-cc-counting-rules.md).
- Coverage joins on the function's line range, not on this identity.
- Whether a header definition is visited once, whether a decorator-only edit marks the function changed, and whether test modules are scored stay on their own tickets.
