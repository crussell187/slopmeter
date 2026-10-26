# What counts as a function, per language

Type: grilling
Status: resolved
Blocked by: 

## Question

Which syntactic units does slopmeter score as a function in each language, and which are excluded or folded into their parent? Python: module-level defs, methods, nested defs, lambdas, async defs, decorated functions (spec says decorators are excluded from CC — but is the decorator call itself scored?), properties. Rust: free fns, impl methods, trait default methods, closures (standalone vs inline callbacks), fns inside macros, `const fn`, generic fns, test fns. C++: free functions, member functions (in-class and out-of-line), constructors/destructors (crap4java excludes constructors), operators, lambdas, templates (per definition, not per instantiation), functions defined in headers. For each language: does a closure or lambda get its own CC and LLOC, or count toward its enclosing function? How is a function identified (the `Function.id` in the IR) stably across runs — qualified name plus signature, file plus line, or a hash?

## Answer

Resolved 2026-10-26 with Christopher. The boundary is the glossary term **Function** in `CONTEXT.md`. Why the lambda rule and the identity rule were chosen: [ADR 0002](../../../docs/adr/0002-what-counts-as-a-function.md).

A function is a named definition with a body the programmer wrote, including one nested inside another. A lambda or closure inside a function belongs to that function, inline or stored in a local. A lambda or closure bound to a name outside any function is a function. A lambda with neither a name nor an enclosing function is unscored.

These are functions: module-level definitions, nested definitions, methods, `async` definitions, properties, free functions, impl methods, trait methods with a default body, `const fn`, generic functions, constructors, destructors, and operators when the body was written out. A `#[test]` function is a function. One generic or template definition is one function, not one per instantiation.

These are left out: a declaration or prototype, a trait method with no default, an `@overload` signature with no body, `= default`, and `= delete`. Decorators and attributes sit above the `def` or `fn` line and add nothing to length, complexity, nesting, or calls. The function runs from that line through the end of its body. A function written inside a Rust macro is invisible: tree-sitter leaves macro bodies opaque ([tree-sitter grammars for Python, Rust and C++ from Rust](01-tree-sitter-grammars.md)).

Identity is the path of the file that holds the body, plus the name qualified inside that file (`A.run.run`, `a::Foo::bar`). C++ adds parameter types and `const` or ref qualifiers, and leaves out the return type and parameter names. Python and Rust do not overload, so the qualified name is enough. The start line breaks a tie only when that qualified name occurs twice in one file. Coverage joins on the line range, not on this identity.

Left on their own tickets: whether a header definition is visited once ([File extensions, dialects and headers per language](08-language-surface.md)); whether a decorator-only edit marks the function changed ([What --changed means](12-changed-semantics.md)); whether test modules are scored ([Source selection and test-file policy](09-source-selection.md)); exemptions for abstract functions ([Pass-through and Demeter detection without symbol resolution](13-passthrough-demeter-exemptions.md)).
