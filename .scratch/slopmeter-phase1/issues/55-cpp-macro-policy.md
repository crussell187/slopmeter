# How C++ function-like macros are surfaced or expanded

Type: grilling
Status: open
Blocked by: 52

## Question

ADR 0012 scores C++ as text, so `#define CHECK(x) if (!(x)) return false;` hides any number of branches with no signal, and once the unparsed ratio is reported it still says nothing about macro density (review, C++ fidelity step 8). Which of these does Phase 1 adopt: an opt-in preprocessing mode that reads `compile_commands.json` when the repo has one and scores the active arm; a per-function note or measurement of unexpanded function-like macro invocations in the body; a rule that flags a touched function whose body contains an unexpanded function-like macro above a count; or nothing beyond the ratio? The answer also decides whether a C++ mutation gate (which needs a compilation database) is possible.
