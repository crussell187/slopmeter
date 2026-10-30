# Whether an empty or swallowing catch ships as F15

Type: grilling
Status: resolved
Blocked by: 

## Question

The spec gates F15, a catch with no rethrow, no return and no log, and schedules it with the Phase 3 additions, which the map lists out of scope. It is the shortcut an agent takes most: `except Exception: pass`, `catch (...) {}`, and in Rust a `Result` discarded with `let _ =` or `.ok()` and never read. The rule needs one pass over a function body and no symbol resolution, and it is a gate in the spec. Decide whether it ships in this effort, what each language's swallowing shape is, what counts as handling (a log call, a return, a re-raise, a comment alone?), whether a `# noqa`-style allowance is the ordinary waiver, and whether it gates or warns. The count is [Score an empty or swallowing catch](84-score-empty-catch.md).

## Answer

Resolved 2026-10-29 with Christopher. F15 ships as a weight-10 gate in all three languages with no warn level and the ordinary waiver. The shape is a handler whose body does nothing: `pass`, `...`, `continue`, `break` or comments in Python; an empty `catch` in C++; `let _ = <call>;`, `.ok();` and an empty `Err` arm or block in Rust. An assignment, a call, a `return`, a `raise`, a `throw;` or a `?` is handling. No config key. [ADR 0037](../../../docs/adr/0037-a-swallowing-handler-is-a-gate.md). The build slice is [Score an empty or swallowing catch](84-score-empty-catch.md).

## Comments

2026-10-29 — F15 ships in this effort as a gate at weight 10 in Python, Rust and C++, with no warn level and the ordinary waiver.

2026-10-29 — The shape is a handler whose body does nothing. Python: an `except` whose body is only `pass`, `...`, `continue`, `break`, or comments. C++: a `catch` whose body is empty apart from comments, or only `continue;` or `break;`. Rust: `let _ = <call>;`, an expression statement ending in `.ok();`, and an `Err(_)` or `Err(name)` arm or `if let Err` block whose body is empty, `()`, or `{}`. An assignment, a call, a `return`, a `raise`, a `throw;` or a `?` is handling. One finding per function at the first swallowing handler.

2026-10-29 — A comment alone in the handler is still swallowing. The config takes no key. The fix shape names the line and says re-raise, return, or handle the error there. The anti-fix is a comment or `...` in place of `pass`.
