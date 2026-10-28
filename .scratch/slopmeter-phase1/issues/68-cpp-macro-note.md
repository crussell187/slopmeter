# Note unscored C++ function-like macros

Type: task
Status: resolved
Blocked by: 55

## Question

Implement the file note in [C++ is scored as text, and both arms of a conditional count](../../../docs/adr/0012-cpp-scored-as-text.md). `File::unscored_macros` counts function-like `#define`s outside a literal-`0` arm. A count above 0 becomes a note and does not change the verdict. An object-like `#define` adds nothing. The code-level plan is [Note unscored C++ function-like macros](../../../docs/superpowers/plans/2026-10-27-cpp-macro-note.md).

## Answer

`adapters::cpp::parse` sets `unscored_macros` to the number of function-like `#define`s outside an arm whose condition is the token `0`. An object-like `#define` adds nothing. A count above 0 becomes a note on that file: one body uses "The parser left 1 function-like macro body unscored." and a larger count uses the plural. The note does not change the verdict. `CHECK(x)` adds no decision on the caller. `CHECK(a && b)` counts the `&&` written in the argument.
