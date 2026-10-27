# Score C++ member-init lists, function-try-block constructors, and macro-wrapped blocks

Type: task
Status: open
Blocked by: 

## Question

Three C++ body-hiding routes produce no ERROR node (review finding 6): a constructor with a function-try-block is not found at all because `written_body` at `src/adapters/cpp.rs:232` expects the `try_statement` in the `body` field; `classify` at `:719` never walks the `field_initializer_list`, so a ternary or an IIFE lambda there is unscored; and `visit` at `:68` and `visit_function` at `:145` recurse into any nested `function_definition`, so `SECTION(x) { if…; if…; }` becomes a phantom nested function and its branches leave the parent. Accept a `try_statement` sibling when `body` is absent, walk the initializer list, and fold a nested definition whose declarator has no typed parameters back into its parent. Fixtures 05, 15, 16 and 17 in the review's C++ artifacts give the expected counts.
