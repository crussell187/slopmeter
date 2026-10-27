---
status: accepted
date: 2026-10-26
---

# An adapter parses one file and returns one file

Each language adapter is one function, `parse(path, source) -> File`, called once per file the scored set already selected. The pipeline reads the bytes. The adapter stores the repo-relative path and does not open it. The source is a byte slice, so a C++ file that is not UTF-8 still parses. A Python encoding cookie is handled inside the Python adapter. Every input returns a file record: an empty file, a syntax error, and a partial tree. Functions come back in source order. `unparsed_ratio` is the only parse signal. Broken lines are not listed, and the ratio does not fail the run.

The adapter does not receive other files, the coverage report, or names a project adds to the allow-list. It publishes that language's default allow-list beside `parse`. A call in the body survives as a member hop, or as `forward` when the body is one forwarding call. There is no edge type, so an `#include`, an import, and a `mod` item are not returned. A header included from many translation units is one call, because source selection already kept that path once. The child of `mod foo` and the target of a Python import are scored only when their own files are selected. Names stay qualified inside the file.

`Language` is an enum. Adding a language adds a variant, a new `parse`, and one arm in the composition root that maps that language's extensions to that function. Analyzers, the rule engine, and the reporters do not gain a language match. The allow-list lookup is the enum.

## Considered options

- **One trait method over every selected file of that language.** Rejected. The adapter could see `#include`, `mod`, and relative imports, and then had to drop them. A name, a span, and a hop come from the file that holds the text.
- **A shared walker driven by a table of node kinds.** Rejected. C++ parameter types, Rust trait-impl names, which Python parameter is the receiver, and a literal `#if 0` are still code. A missing decision kind scores the function as simpler than it is.
- **A plugin registry.** Rejected. A forgotten row scores nothing. An exhaustive match on the enum does not compile.

## Consequences

- Coverage is joined later, on the body span. Chain depth and the forward ratio are derived later. Thresholds stay in the rule engine.
- The composition root is the one place that knows which extensions exist. That list is the language-surface decision, written as the match.
