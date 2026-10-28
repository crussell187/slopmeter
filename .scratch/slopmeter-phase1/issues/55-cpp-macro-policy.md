# How C++ function-like macros are surfaced or expanded

Type: grilling
Status: resolved
Blocked by: 52

## Question

ADR 0012 scores C++ as text, so `#define CHECK(x) if (!(x)) return false;` hides any number of branches with no signal, and once the unparsed ratio is reported it still says nothing about macro density (review, C++ fidelity step 8). Which of these does Phase 1 adopt: an opt-in preprocessing mode that reads `compile_commands.json` when the repo has one and scores the active arm; a per-function note or measurement of unexpanded function-like macro invocations in the body; a rule that flags a touched function whose body contains an unexpanded function-like macro above a count; or nothing beyond the ratio? The answer also decides whether a C++ mutation gate (which needs a compilation database) is possible.

## Answer

Resolved 2026-10-27 with Christopher. Why a function-like `#define` is noted on its file: [ADR 0012](../../../docs/adr/0012-cpp-scored-as-text.md). The glossary term is **Function** in `CONTEXT.md`. The build is [Note unscored C++ function-like macros](68-cpp-macro-note.md).

Phase 1 still scores each C++ file as text. It does not run the preprocessor and does not read `compile_commands.json`. A function-like `#define` is not a function. Its body stays unscored, and a use site scores only the arguments written there.

A file with one or more function-like `#define`s gets a note, in the same family as the unparsed-ratio note: how many of those macros have bodies that were not scored. A file with none omits the note. The note does not change the verdict. An object-like `#define` stays out of the count. A define in an arm whose condition is the token `0` stays out too. A define in any other arm counts.

A C++ mutation gate stays possible. Slopmeter reads the mutation report and does not open the compilation database. [Mutation testing as a slopmeter gate](61-mutation-testing.md) still decides whether this effort includes that gate.
