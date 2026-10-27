# Freeze the Phase 1 IR schema

Type: prototype
Status: resolved
Blocked by: 01, 02, 06, 08, 15

## Question

Write the concrete IR schema (JSON) for Phase 1 and try it on one real file per language. The spec sketches `Module { id, path, exports[], imports[], classes[], functions[] }`, `Function { id, owner, params, lloc, cc, cognitive, nesting, body_calls[], returns_value, mutates_params, is_passthrough }`, `Class { methods[], fields[], field_access_matrix }`, plus `CallEdge`, `ImportEdge`, `CoverageEntry`, `ChurnEntry`. Which of these does Phase 1 populate (function scope only, but the spec says the IR freezes for functions and modules), what is the exact field set and typing for `Function` given the function-identity decision, how are source spans represented, how is per-language information (Rust closures, C++ templates, Python decorators) carried without leaking language into the schema, and how is the schema versioned? Does the IR get serialized to disk in Phase 1?

## Answer

Resolved 2026-10-26 with Christopher. Why the document is files and functions, kept in memory: [ADR 0015](../../../docs/adr/0015-ir-is-files-and-functions.md). The glossary terms are **IR**, **Body span**, and **Touch span** in `CONTEXT.md`. Identity is in [ADR 0002](../../../docs/adr/0002-what-counts-as-a-function.md). The sample is [prototype/ir-schema](../../../prototype/ir-schema/README.md). Its `version` field was dropped.

Phase 1 IR is the scored files and their functions. Classes, exports, imports, call edges, import edges, churn, and coverage stay out. Coverage remains in the LCOV report and joins on the body span. The value lives in memory for the run. It is not written to disk. There is no version integer. The Rust types are the schema.

A file has its repo-relative path, one language tag (`python`, `rust`, or `cpp`), `unparsed_ratio`, and its functions. The tag is the only language-shaped field on a file, so a chain hop can be matched against that language's allow-list.

A function has:

- `name`, one string qualified inside the file. Python is dotted (`get`, `Response.json`, `outer.inner`). An inherent Rust method is `Take::into_inner`. A trait method is `<Take<T> as Buf>::chunks_vectored`. C++ is `CLI::detail::join`.
- `param_types`, a list of strings. Empty for Python and Rust. C++ parameter types only, so the two `trim` overloads are `["std::string &"]` and `["std::string &", "const std::string &"]`.
- `qualifier`, a string. Empty except for a C++ cv and ref qualifier such as `const` or `&&`. The return type and the parameter names stay out.
- `span`, the body span: inclusive line numbers from the `def` or `fn` line through the end of the body. No columns.
- `touch`, the touch span: the body span extended up through a leading decorator or non-doc attribute. A docstring, `///`, `//!`, a handwritten `#[doc]`, and `//` or `/* */` do not extend it. A blank line between an attribute and the definition is inside the range. An attribute on the struct, the impl, or the module is not on the function.
- `params`, each `{name, receiver}`.
- `body`, always present. A declaration is not a function and is omitted.
  - `lloc`, an integer. What increments it is open: [What increments a logical line](25-lloc-counting.md).
  - `decisions`, `{line, depth}`. Depth 1 is a decision not inside another. Cyclomatic complexity is one plus the number of lines. Nesting is the deepest depth, or 0 when there are none.
  - `chains`, `{line, rooted, members: [{name, call}]}`. Depth drops an allow-listed name and a trailing run of calls that ends in `build`.
  - `forward`, null unless the body is one call. Then `{line, names}` lists the ordinary parameters that call received unchanged. The ratio is those names over the ordinary parameters. The 80% line is a threshold.

A closure or lambda inside a function is not its own record. Its decisions sit on the function that contains it. A template or generic is one function, and the written type parameter stays in `param_types` or in the name. Decorators and attributes are not a list. Cognitive complexity has no field until [Cognitive complexity increments](24-cognitive-complexity-increments.md) closes.

The same name, parameter types, and qualifier twice in one file is the only tie, broken by the earlier body span. That start line is not a separate field.

The sample encoded `src/requests/api.py` in full, and the awkward functions in `src/buf/take.rs` and `include/CLI/StringTools.hpp`. `get` forwards three of three ordinary parameters. `head` has two statements, so `forward` is null. `trim` returns `ltrim(rtrim(str))`, so `forward` is null. On `chunks_vectored`, `dst.len().min(...)` is depth 2 and does not start at the receiver. `iter_mut` and `zip` drop out on the allow-list. In `join`, `empty`, `size`, and `back` drop out, and `str()` and `pop_back()` stay at depth 1.
