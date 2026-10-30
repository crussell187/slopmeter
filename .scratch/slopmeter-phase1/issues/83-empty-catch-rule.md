# Whether an empty or swallowing catch ships as F15

Type: grilling
Status: open
Blocked by: 

## Question

The spec gates F15, a catch with no rethrow, no return and no log, and schedules it with the Phase 3 additions, which the map lists out of scope. It is the shortcut an agent takes most: `except Exception: pass`, `catch (...) {}`, and in Rust a `Result` discarded with `let _ =` or `.ok()` and never read. The rule needs one pass over a function body and no symbol resolution, and it is a gate in the spec. Decide whether it ships in this effort, what each language's swallowing shape is, what counts as handling (a log call, a return, a re-raise, a comment alone?), whether a `# noqa`-style allowance is the ordinary waiver, and whether it gates or warns. The count is [Score an empty or swallowing catch](84-score-empty-catch.md).
