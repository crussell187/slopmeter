---
status: accepted
date: 2026-10-29
---

# A swallowing handler is a gate

When a test fails, the cheapest edit is to wrap the call and swallow the error. Every static metric then improves. F15 scores a handler whose body does nothing, as a gate at weight 10 in Python, Rust and C++, with no warn level. In Python it is an `except` whose body is only `pass`, `...`, `continue`, `break`, or comments. In C++ it is a `catch` whose body is empty apart from comments, or only `continue;` or `break;`. Rust has no catch, so it is `let _ = <call>;`, an expression statement that ends in `.ok();`, and an `Err(_)` or `Err(name)` arm or `if let Err` block whose body is empty, `()`, or `{}`. An assignment, a call, a `return`, a `raise`, a `throw;`, or a `?` is handling, and that handler is not scored. One finding per function sits at the first swallowing handler. A comment alone inside the handler is still swallowing; the waiver comment above the function is the only excuse. The config takes no key.

## Considered options

- **The spec's shape: no rethrow, no return, no log.** Rejected for a gate. `except KeyError: value = None` sets a default, and a gate that fails it would be waived everywhere.
- **A warn.** Rejected. A warn moves the score and not the verdict, and this is the edit the loop exists to refuse.
- **Leave it with the Phase 3 additions.** Rejected. Every language here has a syntactic form that needs no symbol resolution, and the sloppy corpora already fail on every other gate.
- **Count a handler that only logs.** Rejected. A log is handling in the spec's own words, and whether the log is enough is a judgement the tool does not make.

## Consequences

- The fix shape names the line and says re-raise, return, or handle the error there. The anti-fix is a comment or `...` in place of `pass`, which the shape still catches.
- `explain F15` is a new page. [A finding's weight is fixed](0025-finding-weight-is-fixed.md) gains a weight-10 gate. [Each function rule has one metric](0022-rule-ids-and-stand-in-thresholds.md) gains F15 with the threshold "any".
- The count is [Score an empty or swallowing catch](../../.scratch/slopmeter-phase1/issues/84-score-empty-catch.md).
