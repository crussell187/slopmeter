# Freeze the Phase 1 IR schema

Type: prototype
Status: open
Blocked by: 01, 02, 06, 08, 15

## Question

Write the concrete IR schema (JSON) for Phase 1 and try it on one real file per language. The spec sketches `Module { id, path, exports[], imports[], classes[], functions[] }`, `Function { id, owner, params, lloc, cc, cognitive, nesting, body_calls[], returns_value, mutates_params, is_passthrough }`, `Class { methods[], fields[], field_access_matrix }`, plus `CallEdge`, `ImportEdge`, `CoverageEntry`, `ChurnEntry`. Which of these does Phase 1 populate (function scope only, but the spec says the IR freezes for functions and modules), what is the exact field set and typing for `Function` given the function-identity decision, how are source spans represented, how is per-language information (Rust closures, C++ templates, Python decorators) carried without leaking language into the schema, and how is the schema versioned? Does the IR get serialized to disk in Phase 1?
