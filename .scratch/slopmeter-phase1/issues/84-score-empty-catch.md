# Score an empty or swallowing catch

Type: task
Status: resolved
Blocked by: 

## Question

Build F15 per [ADR 0037](../../../docs/adr/0037-a-swallowing-handler-is-a-gate.md). Each adapter records on the IR function the line of each handler whose body does nothing: Python an `except` whose body is only `pass`, `...`, `continue`, `break`, or comments; C++ a `catch` whose body is empty apart from comments or only `continue;` or `break;`; Rust `let _ = <call>;`, an expression statement ending in `.ok();`, and an `Err(_)` or `Err(name)` arm or `if let Err` block whose body is empty, `()`, or `{}`. `rules::evaluate` emits one weight-10 gate per function at the first such line with the fix shape and anti-fix from the ADR, the ordinary waiver applies, `explain F15` is written, the goldens gain the rule, and each sloppy corpus gains one swallowing handler so the validation tests fail on it.

## Answer

Resolved 2026-10-30. Each adapter records every swallowing handler's line on `Body::swallowing`, and `analyzers::measure` copies that list. `rules::evaluate` emits one F15 gate at the first line. The measured value is the count of those lines and the threshold is 0, so any handler fails and a waiver holds when its number is at least that count. The fix shape names the line and says re-raise, return, or handle the error there. The anti-fix is a comment or `...` in place of `pass`. `explain F15` says the threshold is any, and `[profile.*.F15]` is an unknown key. The goldens include `tests/rules/F15.py`. Each sloppy `hold` function gained one swallowing handler; the wide-margin check accepts that one handler at 1 against 0, because a threshold of 0 cannot be cleared by one and a half times itself. `slopmeter --strict` on this repo still exits 0.
