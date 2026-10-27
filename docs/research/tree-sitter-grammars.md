# tree-sitter grammars for Python, Rust and C++ from Rust

Resolves `.scratch/slopmeter-phase1/issues/01-tree-sitter-grammars.md`. Researched 2026-10-26 against crates.io, docs.rs, the tree-sitter GitHub org and tree-sitter.github.io only.

## Answer

Use the core crate `tree-sitter` 0.27.1 plus the three org-maintained grammar crates `tree-sitter-python` 0.25.0, `tree-sitter-rust` 0.24.2 and `tree-sitter-cpp` 0.23.4 (all under `github.com/tree-sitter/`, MIT, authored by Max Brunsfeld and Amaan Qureshi). Grammar crates do not depend on `tree-sitter` at all; they expose a `LANGUAGE: LanguageFn` from `tree-sitter-language` 0.1, which converts into `tree_sitter::Language` via `From`. Compatibility is by ABI number: core 0.27.1 accepts ABI 13..=15, and the published parsers are ABI 15 (Python, Rust) and ABI 14 (C++), so all three load. Every construct the function-scope rules need has a named node type (table below); the two things the grammars handle badly are Rust macro bodies (opaque `token_tree`) and C++ preprocessor conditionals/macros, which the C++ grammar parses as block-level items in a way that splits statements and reuses the ordinary `binary_expression`/`call_expression` names inside `#if` conditions. tree-sitter-cpp's README documents no limitations at all; the evidence is grammar.js comments and open issues. Queries are S-expression patterns compiled once per language (`Query::new`), run by a `QueryCursor` whose `matches`/`captures` return streaming iterators (`StreamingIterator` must be in scope). Partial parses never fail: the tree is complete and carries `ERROR` nodes (unparseable spans) and zero-width `MISSING` nodes (tokens the parser inserted), checkable with `Node::is_error`, `is_missing`, `has_error` or queried with `(ERROR)` / `(MISSING)`.

## Crates, versions, pins, maintainers

| Crate | Latest on crates.io | Published | Runtime deps | `tree-sitter` dev-dep | Parser ABI | Repo |
|---|---|---|---|---|---|---|
| `tree-sitter` | 0.27.1 | 2026-10-08 | `streaming-iterator ^0.1.9`, ... | n/a | supports 13..=15 | tree-sitter/tree-sitter |
| `tree-sitter-language` | 0.1.9 | 2026-10-08 | none | n/a | n/a | tree-sitter/tree-sitter |
| `tree-sitter-python` | 0.25.0 | 2025-09-11 | `tree-sitter-language ^0.1` | `^0.25.8` | 15 | tree-sitter/tree-sitter-python |
| `tree-sitter-rust` | 0.24.2 | 2026-03-27 | `tree-sitter-language ^0.1` | `^0.25` | 15 | tree-sitter/tree-sitter-rust |
| `tree-sitter-cpp` | 0.23.4 | 2024-11-11 | `tree-sitter-language ^0.1` | `^0.24` | 14 | tree-sitter/tree-sitter-cpp |

Facts and where they come from:

- The grammar crates pin **no** core version. Their only normal dependency is `tree-sitter-language ^0.1` (crates.io dependency endpoints for each published version). `tree-sitter-language` is "The tree-sitter Language type, used by the library and by language implementations"; its single type `LanguageFn` "wraps a C function that returns a pointer to a tree-sitter grammar", with `unsafe fn from_raw` / `fn into_raw`. `tree_sitter::Language` implements `From<LanguageFn>`, hence the documented idiom `parser.set_language(&tree_sitter_python::LANGUAGE.into())`.
- Compatibility is therefore the ABI check, not Cargo semver. docs.rs for `tree-sitter` 0.27.1 gives `LANGUAGE_VERSION = 15` ("The latest ABI version that is supported by the current version of the library") and `MIN_COMPATIBLE_LANGUAGE_VERSION = 13`; `api.h` on master defines the same two numbers. `Parser::set_language` "fails if the language cannot be used for parsing", including "if it was generated with an incompatible version of the Tree-sitter CLI"; `Language::abi_version()` returns the generating CLI's ABI. The `#define LANGUAGE_VERSION` in each tag's `src/parser.c` is 15 for python v0.25.0 and rust v0.24.2, and 14 for cpp v0.23.4.
- All three grammar crates publish from the `tree-sitter` GitHub org, list Brunsfeld and Qureshi as authors (Rust adds Maxim Sokolov; C++ adds John Drouhard in `tree-sitter.json`), and are MIT.
- `tree-sitter-cpp` is the stale one: crates.io 0.23.4 dates from 2024-11 and was generated with CLI 0.24 (ABI 14). Its `master` branch is still versioned 0.23.4 in `Cargo.toml` but is already ABI 15, has a `tree-sitter ^0.26` optional dep, and adds C++20 modules / C++26 reflection nodes (`module_declaration`, `import_declaration`, `splice_expression`, `lambda_declarator`, `explicit_object_parameter_declaration`, ...). The table below was verified against the **tagged** node-types.json, not master. Python v0.25.0 vs master differ only by one node (`tuple_expression`); Rust v0.24.2 equals master.
- Each grammar crate also exports `NODE_TYPES` (the `node-types.json` text), `TAGS_QUERY` and a highlights query (`HIGHLIGHTS_QUERY` for Python/Rust, `HIGHLIGHT_QUERY` for C++; Rust also `INJECTIONS_QUERY`).

## Node types per construct

Source for every row: `src/node-types.json` at the tagged release, cross-checked with `grammar.js`. Field names are shown as `field:`; "child" means an unfielded named child. The maintainers' own `queries/tags.scm` corroborate the definition rows: Python tags `(function_definition name: (identifier))`, Rust tags `(function_item ...)` and `(declaration_list (function_item ...)) @definition.method`, C++ tags `(function_declarator declarator: ...)` for functions and methods.

### Function and method definitions

| Construct | Python 0.25.0 | Rust 0.24.2 | C++ 0.23.4 |
|---|---|---|---|
| Function | `function_definition` (`name:`, `parameters:`, `body:`, `return_type:`) | `function_item` (`name:`, `parameters:`, `body:`, `return_type:`) | `function_definition` (`type:`, `declarator:` a `function_declarator` chain, `body:` `compound_statement` or `try_statement`) |
| Method | `function_definition` nested in `class_definition` `body:` `block` | `function_item` inside `impl_item`/`trait_item` `body:` `declaration_list` | inline: `function_definition` inside `field_declaration_list`; out-of-line: `function_definition` whose `function_declarator` `declarator:` is `qualified_identifier`; declaration-only: `field_declaration` wrapping `function_declarator` |
| Trait default vs required | n/a | default body: `function_item`; signature only: `function_signature_item` (no `body:`) | pure virtual: `function_definition`/`field_declaration` with `pure_virtual_clause` child |
| Closure / lambda | `lambda` (`parameters:` `lambda_parameters`, `body:` `expression`) | `closure_expression` (`parameters:` `closure_parameters`, `body:` expression or block) | `lambda_expression` (`captures:`, `declarator:` `abstract_function_declarator` holding `parameters:`, `body:` `compound_statement`, `template_parameters:`) |
| Nested def | `function_definition` at any depth; decorated ones wrapped in `decorated_definition` (`definition:`) | `function_item` inside `block` (`_declaration_statement`) | not a C++ feature; local lambdas only |
| Template | n/a | generics are `type_parameters:` on `function_item` | `template_declaration` (`parameters:` `template_parameter_list`) whose child is `function_definition`/`declaration`/nested `template_declaration` |
| Constructor / destructor | `function_definition` named `__init__` | n/a | grammar rule `constructor_or_destructor_definition` is **aliased to `function_definition`** (no `type:` field; may carry `field_initializer_list`); declarations alias to `declaration`; destructor name is `destructor_name` |
| Operator overload | `function_definition` (dunder) | `function_item` in `impl_item` | `function_definition` whose declarator name is `operator_name` or `operator_cast` |
| Async / other bodies | `function_definition` (`async` is an anonymous token) | `async_block`, `unsafe_block`, `try_block`, `const_block` wrap a `block` | `function_definition` `body:` may be a `try_statement` (function-try-block) |

### Branch constructs for cyclomatic complexity

| Construct | Python | Rust | C++ |
|---|---|---|---|
| if / elif / else | `if_statement` (`condition:`, `consequence:`, `alternative:` = `elif_clause` or `else_clause`) | `if_expression` (`condition:` expr, `let_condition` or `let_chain`; `alternative:` `else_clause` whose child is `block` or another `if_expression`) | `if_statement` (`condition:` `condition_clause` with `value:` and optional `initializer:`; `alternative:` `else_clause`) |
| Loops | `for_statement`, `while_statement` (both have `alternative:` `else_clause`) | `for_expression`, `while_expression`, `loop_expression` | `for_statement`, `for_range_loop`, `while_statement`, `do_statement` |
| Match / switch | `match_statement` with `case_clause` children; guard is `guard:` `if_clause` | `match_expression` -> `body:` `match_block` -> `match_arm` (`pattern:` `match_pattern`, which holds `condition:` for guards) | `switch_statement` -> `case_statement` (`value:` present for `case`, absent for `default`) |
| `?` / exceptions | `try_statement` with `except_clause`, `else_clause`, `finally_clause` children (no separate `except*` node type at this version) | `try_expression` (the `?` operator) | `try_statement` with `catch_clause` children |
| Ternary | `conditional_expression`: three positional `expression` children, **no fields** (grammar: `expr 'if' expr 'else' expr`) | `if_expression` used as an expression | `conditional_expression` (`condition:`, `consequence:`, `alternative:`) |
| Short-circuit ops | `boolean_operator` with `operator:` `and` / `or` | `binary_expression` with `operator:` `&&` / `\|\|` | `binary_expression` with `operator:` `&&`, `\|\|`, **and the alternative tokens `and`, `or`** |
| Comprehension filter | `if_clause` child of `list_comprehension`, `set_comprehension`, `dictionary_comprehension`, `generator_expression` | n/a | n/a |
| Walrus / let-else | `named_expression` | `let_condition`, `let_chain` | `condition_clause` `initializer:` `init_statement` |

### Parameters, calls, member access, comments

| Construct | Python | Rust | C++ |
|---|---|---|---|
| Parameter list | `parameters` / `lambda_parameters` (children are the `parameter` supertype) | `parameters` (children `parameter`, `self_parameter`, `variadic_parameter`); `closure_parameters` | `parameter_list` (children `parameter_declaration`, `optional_parameter_declaration`, `variadic_parameter_declaration`) |
| Call | `call` (`function:`, `arguments:` `argument_list` or `generator_expression`) | `call_expression` (`function:`, `arguments:` `arguments`); `macro_invocation` (`macro:`) | `call_expression` (`function:`, `arguments:` `argument_list`) |
| Member access | `attribute` (`object:`, `attribute:`) | `field_expression` (`value:`, `field:`); method calls are `call_expression` whose `function:` is a `field_expression`; `scoped_identifier` (`path:`, `name:`) | `field_expression` (`argument:`, `field:`, `operator:` `.` / `->` / `.*`); `qualified_identifier` (`scope:`, `name:`); `template_function`, `template_method` |
| Comment | `comment` (extra) | `line_comment`, `block_comment` (extras; `doc:` field marks doc comments) | `comment` (extra, inherited from tree-sitter-c) |

Comments are declared in each grammar's `extras`, so they can appear between any two siblings; `Node::is_extra()` identifies them and the `.`-anchor in queries ignores anonymous nodes but **not** named extras such as comments.

Supertypes worth knowing for queries: Python `expression`, `primary_expression`, `parameter`, `pattern`; Rust `_expression`, `_pattern`, `_type`, `_declaration_statement`; C++ `expression`, `statement`, `type_specifier`, `_declarator`. Supertypes "are hidden from the syntax tree" but "can still be used in queries", and the `supertype/subtype` form (e.g. `(expression/binary_expression)`) is available.

## The query API from Rust

- A query is "a set of patterns that match nodes in a syntax tree", written as S-expressions: `(binary_expression (number_literal) (number_literal))`; `field: (node)`; `!field` negation; `"=="` for anonymous nodes; `(_)` any named node, `_` any node; `@capture`; `+ * ?` quantifiers; `[a b]` alternations; `(...)` sibling groups; `.` anchors (first/last named child or immediate siblings). Pattern syntax and operators are documented on the query syntax and operators pages.
- Predicates and directives "are not handled directly by the Tree-sitter C library"; bindings implement a few. The Rust binding source (`lib/binding_rust/lib.rs`) evaluates `eq?`/`not-eq?`/`any-eq?`/`any-not-eq?`, `match?`/`not-match?`/`any-match?`/`any-not-match?`, `any-of?`/`not-any-of?`, and records `is?`/`is-not?` (via `property_predicates`) and `set!` (via `property_settings`); anything else lands in `Query::general_predicates(index)` for the caller.
- `Query::new(&Language, &str) -> Result<Query, QueryError>` compiles once per language (query-time errors are reported with an offset and a kind: syntax, node type, field or capture). `QueryCursor::new()`, then `cursor.matches(&query, node, text_provider)` or `cursor.captures(...)`; the text provider is `&[u8]` source. `matches` yields `QueryMatch { pattern_index, captures }` in match order; `captures` yields individual captures in document order. Both return streaming iterators: "Iterating over a `QueryMatches` object requires the `StreamingIterator` or `StreamingIteratorMut` trait to be in scope" (`streaming-iterator` 0.1.9 is a dependency). Scope a run with `set_byte_range` / `set_point_range` (intersecting) or `set_containing_byte_range` (fully contained), bound depth with `set_max_start_depth(Option<u32>)`, and cap work with `set_match_limit` + `did_exceed_match_limit`. `Query` is immutable and "can be safely shared between threads"; a cursor is not.
- For manual walking: `Node::walk()` gives a `TreeCursor` ("the fastest way" to visit many nodes) with `goto_first_child` / `goto_next_sibling` / `goto_parent` returning `bool` and `field_name()` for the current node; `Node::child_by_field_name`, `children_by_field_name(name, &mut cursor)`, `named_children(&mut cursor)`, `kind()`, `kind_id()`, `is_named()`, `utf8_text(&[u8])`, `start_position()`/`end_position()` (0-based row, byte column), `byte_range()`.
- Parsing: `Parser::new()`, `set_language(&lang) -> Result<(), LanguageError>`, `parse(text, None) -> Option<Tree>` (`None` only when no language is set, or on cancellation via `parse_with_options`), `set_included_ranges(&[Range])` for sub-ranges, `tree.root_node()`. `Tree` instances are "not thread safe" but copies are cheap and may be used on other threads.

## ERROR and MISSING on partial parses

Tree-sitter always returns a complete tree. Unrecognisable spans become `ERROR` nodes; "Missing nodes are inserted by the parser in order to recover from certain kinds of syntax errors" and are zero-width. From Rust: `Node::is_error()` ("represents a syntax error"), `Node::is_missing()`, `Node::has_error()` ("represents a syntax error or contains any syntax errors anywhere within it"); the grammar crates' own doc examples assert `!tree.root_node().has_error()`. In queries, `(ERROR)` matches error nodes and `(MISSING)` matches inserted tokens, optionally typed as `(MISSING identifier)` or `(MISSING ";")`; the docs warn that `(ERROR)` queries do not capture missing nodes, so both are needed to find all syntax errors. Query patterns can mention `ERROR` as a node type, and node-types.json does not list it (static node types doc). Practical consequence for slopmeter: a function whose body contains an `ERROR` is still a `function_definition`, so metrics should check `has_error()` per function and either skip or flag it rather than assuming a clean tree.

## What tree-sitter-cpp says about the preprocessor and templates

- **The README documents nothing**: it is badges plus two links to C++ BNF references (the C grammar's README likewise). No "limitations" section exists in either repo.
- **Preprocessor handling, from grammar.js** (inherited from tree-sitter-c): `#include`, `#define`, function-like `#define`, and generic directives are `preproc_include`, `preproc_def` (`name:`, `value:` `preproc_arg`), `preproc_function_def` (`parameters:` `preproc_params`), `preproc_call` (`directive:`, `argument:`). `preproc_arg` is a single raw token to end of line (`token(prec(-1, /\S([^/\n]|\/[^*]|\\\r?\n)*/))`), so **macro bodies are never parsed**. Conditionals are built by a `preprocIf(suffix, content)` helper that generates `preproc_if`/`preproc_ifdef` (with `alternative:` `preproc_elif`/`preproc_elifdef`/`preproc_else`) at each nesting site: top level, inside blocks (`_in_block`), inside `field_declaration_list` and inside enumerator lists. Each branch simply repeats the surrounding item rule. Two consequences: (1) a conditional must wrap whole items; a `#if` that splits a statement or expression produces `ERROR` nodes, as in open issue #308 (`int x = \n#ifdef ONE\n 1\n#else\n 2\n#endif\n;` yields three ERRORs); (2) **both branches are in the tree**, so a complexity counter will count `#else` code as if it were live.
- **`#if` conditions reuse ordinary node names.** `_preproc_expression` aliases `preproc_binary_expression` to `binary_expression`, `preproc_call_expression` to `call_expression`, plus `unary_expression`/`parenthesized_expression`; node-types.json confirms `binary_expression.left` may be `preproc_defined`. A query for `(binary_expression operator: "&&")` therefore also matches `#if defined(A) && defined(B)`. Exclude ancestors of type `preproc_if`/`preproc_elif` when counting.
- **Macro invocations in code are not recognised** as such: an unknown identifier is parsed as whatever the grammar can make of it. The C++ grammar deliberately drops C's macro-friendly declarator trick ("C uses _declaration_declarator here for some nice macro parsing in function declarators, but this causes a world of pain for C++ so we'll just stick to the normal _declarator here") and only adds `compound_statement` as a call argument "to parse macros taking statements as arguments". Issue #85 ("Macro prevents correct parsing of class", open since 2020) collects cases where attribute/export macros around `class` produce ERRORs or misparse the class as a function; no maintainer has stated a position.
- **Templates** are supported (`template_declaration`, `template_function`, `template_method`, `template_type`, `template_argument_list`, `requires_clause`, `concept_definition`), with no documented limitation. Open bugs show the usual `<`/`>` ambiguity: #346 "template method calls parsed as two comparison operators", #318 variadic template with pointer args, #192 nested template argument lists. Dependent-name constructs (`typename`, `dependent_name`) exist as node types.

Not confirmable from primary sources: whether Python `except*` parses in 0.25.0 (no `except_group_clause` node exists, only `except_clause`), and whether tree-sitter-cpp's unreleased master will be published as 0.24 or 0.23.5.

## Rust-grammar caveat

`macro_invocation` carries only `macro:` and a `token_tree` child, whose children are literals, identifiers, punctuation and nested `token_tree`s. Code inside `println!`, `vec!`, `assert!`, `tokio::select!` or test-generating macros has no `if_expression`/`call_expression` structure, so metrics silently under-count anything written inside macros. Likewise `function_signature_item` (trait required methods, `extern` blocks) has no body and should not be scored.

## Sources

- https://crates.io/api/v1/crates/tree-sitter
- https://crates.io/api/v1/crates/tree-sitter-language
- https://crates.io/api/v1/crates/tree-sitter-python and https://crates.io/api/v1/crates/tree-sitter-python/0.25.0/dependencies
- https://crates.io/api/v1/crates/tree-sitter-rust and https://crates.io/api/v1/crates/tree-sitter-rust/0.24.2/dependencies
- https://crates.io/api/v1/crates/tree-sitter-cpp and https://crates.io/api/v1/crates/tree-sitter-cpp/0.23.4/dependencies
- https://docs.rs/tree-sitter/latest/tree_sitter/ (plus `struct.Parser.html`, `struct.Language.html`, `struct.Node.html`, `struct.Query.html`, `struct.QueryCursor.html`, `constant.LANGUAGE_VERSION.html`, `constant.MIN_COMPATIBLE_LANGUAGE_VERSION.html`)
- https://docs.rs/tree-sitter-language/latest/tree_sitter_language/ and `struct.LanguageFn.html`
- https://docs.rs/tree-sitter-python/latest/tree_sitter_python/
- https://docs.rs/tree-sitter-rust/latest/tree_sitter_rust/
- https://docs.rs/tree-sitter-cpp/latest/tree_sitter_cpp/
- https://github.com/tree-sitter/tree-sitter — `lib/include/tree_sitter/api.h`, `lib/binding_rust/lib.rs` (master)
- https://github.com/tree-sitter/tree-sitter-python — `Cargo.toml`, `grammar.js`, `tree-sitter.json` (master); `src/node-types.json`, `src/parser.c`, `queries/tags.scm` at tag `v0.25.0`
- https://github.com/tree-sitter/tree-sitter-rust — same files at master and tag `v0.24.2`
- https://github.com/tree-sitter/tree-sitter-cpp — `README.md`, `Cargo.toml`, `grammar.js`, `tree-sitter.json` (master); `src/node-types.json`, `src/parser.c`, `queries/tags.scm` at tag `v0.23.4`; releases page; issues #85, #308 and the issue searches for "preprocessor" and "template"
- https://github.com/tree-sitter/tree-sitter-c — `README.md`, `grammar.js` (master; the C++ grammar extends it)
- https://tree-sitter.github.io/tree-sitter/using-parsers/1-getting-started.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/4-walking-trees.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types.html
- https://tree-sitter.github.io/tree-sitter/using-parsers/queries/index.html, `1-syntax.html`, `2-operators.html`, `3-predicates-and-directives.html`, `4-api.html`
- https://tree-sitter.github.io/tree-sitter/creating-parsers/3-writing-the-grammar.html
