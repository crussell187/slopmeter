# C++ preprocessor and template handling

Type: grilling
Status: resolved
Blocked by: 01

## Question

tree-sitter-cpp parses source without running the preprocessor. Given what the grammar research found: how does the C++ adapter treat `#if/#ifdef` blocks (score all branches, the first, none?), function-like macros that hide branches or whole function bodies, and templates (score the definition once; ignore instantiations)? Does the adapter read `compile_commands.json` for defines and include paths, or stay purely syntactic in Phase 1 and report an `unparsed_ratio` per file so the reader knows how much to trust the numbers?

## Answer

Resolved 2026-10-26 with Christopher. Why C++ is scored as text, with both arms of a conditional counted: [ADR 0012](../../../docs/adr/0012-cpp-scored-as-text.md). The glossary term is **Unparsed ratio** in `CONTEXT.md`. The grammar facts are on [tree-sitter grammars for Python, Rust and C++ from Rust](01-tree-sitter-grammars.md).

Phase 1 parses each C++ file as text. It does not run the preprocessor, and it does not read `compile_commands.json`. Each file is parsed once, `#include` is not followed, and `import` is not resolved.

Every arm of `#if`, `#ifdef`, `#ifndef`, `#elif`, and `#else` counts. Inside a function, the statements belong to it. A definition in an arm is its own function, so two platform bodies of the same name are two functions. The condition adds nothing to complexity. The one ignored arm is a condition that is the token `0`, parentheses allowed. `#if 1` counts. `#ifdef FLAG` counts on every arm. `#if 0 && FLAG` counts too, because Phase 1 does not fold expressions. A `#if` that splits a statement and comes back as an `ERROR` node is not a recovered arm.

A `#define` body is a raw token. It is not a function, and the branches written there count nowhere. Source at the use site is scored with the ordinary rules when it parses, so an argument written in the caller counts and the `if` hidden in the macro does not. A use that does not parse is an `ERROR` or `MISSING` node. The adapter scores the tree it recovered, including a function that contains an error.

A template is one function per written definition. An explicit specialization is its own function, because its body was written out. An instantiation is not in the text, so it is not scored.

`unparsed_ratio` is the fraction of lines in the file that an `ERROR` or `MISSING` node touches. It is a trust signal, reported for Python and Rust as well as C++. Phase 1 has no cutoff that drops the file or fails the run.
