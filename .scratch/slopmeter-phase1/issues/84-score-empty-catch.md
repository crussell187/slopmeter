# Score an empty or swallowing catch

Type: task
Status: open
Blocked by: 

## Question

Build F15 per [ADR 0037](../../../docs/adr/0037-a-swallowing-handler-is-a-gate.md). Each adapter records on the IR function the line of each handler whose body does nothing: Python an `except` whose body is only `pass`, `...`, `continue`, `break`, or comments; C++ a `catch` whose body is empty apart from comments or only `continue;` or `break;`; Rust `let _ = <call>;`, an expression statement ending in `.ok();`, and an `Err(_)` or `Err(name)` arm or `if let Err` block whose body is empty, `()`, or `{}`. `rules::evaluate` emits one weight-10 gate per function at the first such line with the fix shape and anti-fix from the ADR, the ordinary waiver applies, `explain F15` is written, the goldens gain the rule, and each sloppy corpus gains one swallowing handler so the validation tests fail on it.
