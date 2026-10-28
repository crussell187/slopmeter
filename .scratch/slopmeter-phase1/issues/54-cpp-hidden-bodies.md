# Score C++ member-init lists, function-try-block constructors, and macro-wrapped blocks

Type: task
Status: resolved
Blocked by: 

## Question

Three C++ body-hiding routes produce no ERROR node (review finding 6): a constructor with a function-try-block is not found at all because `written_body` at `src/adapters/cpp.rs:232` expects the `try_statement` in the `body` field; `classify` at `:719` never walks the `field_initializer_list`, so a ternary or an IIFE lambda there is unscored; and `visit` at `:68` and `visit_function` at `:145` recurse into any nested `function_definition`, so `SECTION(x) { if…; if…; }` becomes a phantom nested function and its branches leave the parent. Accept a `try_statement` sibling when `body` is absent, walk the initializer list, and fold a nested definition whose declarator has no typed parameters back into its parent. Fixtures 05, 15, 16 and 17 in the review's C++ artifacts give the expected counts.

## Answer

A function-try-block constructor is a function: the `try` sits beside the declarator, and the `if` and the `catch` both count. A member-initializer list counts on that constructor, including a ternary, a `&&`, and a lambda invoked there; the lambda is not its own function. A nested definition with no return type and no typed parameter (`SECTION(first) { … }`, `BOOST_FOREACH(x, v) { … }`) is not a function, and the block's branches stay on the parent. A nested definition with a return type or a typed parameter (`int helper(int x)`) stays its own function, and so does a constructor of a local class. The loop a macro would expand to is still absent; that question stays on [How C++ function-like macros are surfaced or expanded](55-cpp-macro-policy.md). [ADR 0012](../../../docs/adr/0012-cpp-scored-as-text.md) now states the three rules.
