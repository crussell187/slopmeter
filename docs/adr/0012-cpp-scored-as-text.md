---
status: accepted
date: 2026-10-26
---

# C++ is scored as text, and both arms of a conditional count

Phase 1 does not run the preprocessor and does not read `compile_commands.json`. A header is scored once, and the same header is often built with different defines, so one compile command would hide the other builds. Each file is parsed once. `#include` is not followed, and `import` is not resolved.

Every arm of `#if`, `#ifdef`, `#ifndef`, `#elif`, and `#else` counts. The condition adds nothing. The one ignored arm is a condition that is the token `0`, parentheses allowed. A `#define` body is a raw token and is not a function. Source at the use site is scored when it parses. A constructor's member-initializer list and its function-try-block belong to that constructor. A nested definition with no return type and no typed parameter is a macro-wrapped block: it is not a function, and the block counts on the function that contains it. A nested definition that has a return type or a typed parameter stays its own function. The adapter scores the tree it recovered, including a function that contains an error. `unparsed_ratio` is the fraction of lines whose tokens sit directly under an `ERROR` or `MISSING` node. An error whose children are recovered items does not contribute its own span. It is reported for every language, as a note on that file when the ratio is above 0. It does not drop the file or fail the run.

## Considered options

- **Preprocess from `compile_commands.json` and score the active arm.** Rejected. Defines differ by target, and a header scored once cannot be one of those builds.
- **Score only the first arm, or skip a function that contains a conditional.** Rejected. The first arm is often the dead one, and skipping the function lets a `#if` hide the body from the gate.
- **Fail the run when the parse has errors.** Rejected. Real C++ has macro spans the grammar cannot recover. The ratio says how far to trust the file. The functions that did parse still score.

## Consequences

- A template remains one function per written definition. An explicit specialization is its own function. An instantiation is not in the text.
- A `#if` that splits a statement and comes back as `ERROR` is not a recovered arm. It moves the ratio.
- The same ratio and the same "score what was recovered" rule apply to Python and Rust.
