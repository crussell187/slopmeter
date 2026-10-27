# The adapter interface contract

Type: prototype
Status: resolved
Blocked by: 19

## Question

Design the seam every language adapter fills: inputs (selected files, per-language config, coverage map?) and the single output (IR). Use /design-an-interface to compare at least two shapes — e.g. one trait with `parse(file) -> Module` per file vs a whole-project `analyze(files) -> IR` that can see across files — against the three adapters' needs (C++ headers seen from many translation units, Rust module tree, Python packages). How do adapters report unparsed regions and unknown edges? The contract is what makes adding a language touch one adapter and nothing else.

## Answer

Resolved 2026-10-26 with Christopher. Why each adapter is one function over one file: [ADR 0016](../../../docs/adr/0016-adapter-parses-one-file.md). The glossary term is **Adapter** in `CONTEXT.md`.

The contract is one function per language:

```rust
pub fn parse(path: &str, source: &[u8]) -> File
```

The pipeline has already read the file and already chosen it for the scored set. `parse` stores `path` and does not open it. The bytes stay bytes. A Python encoding cookie is handled inside `python::parse`. An empty file, a syntax error, and a partial tree all return a `File`. Functions are in source order. `unparsed_ratio` is the only parse signal. Broken lines are not listed, and the ratio does not fail the run.

`python::parse`, `rust::parse`, and `cpp::parse` share the IR types and do not share a walker. The language tag is an enum with those three variants. The call receives the path and the bytes. It does not receive the coverage report, the other files, or names a project adds. The adapter module publishes that language's default allow-list beside `parse`. The analyzer applies it. Names a project adds are the `allow` key in [Shape of slopmeter.toml](22-config-shape.md).

A call survives as a hop on that body, or as `forward` when the body is one forwarding call. An `#include`, an import, and a `mod` item are not records. A header included from many translation units is one call, because the scored set already kept that path once. The child of `mod foo` and the target of a Python import are scored only when their own files are selected. Names stay qualified inside the file.

Adding a language adds a `parse`, a `Language` variant, and one arm in the composition root that maps that language's extensions to that function. Analyzers, the rule engine, and the reporters do not gain a language match. The allow-list lookup is the enum, so a missing variant fails to compile.
