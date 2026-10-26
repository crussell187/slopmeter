# tree-sitter grammars for Python, Rust and C++ from Rust

Type: research
Status: resolved
Context: research subagent fired 2026-10-26; findings land on branch `research/tree-sitter-grammars` at `docs/research/tree-sitter-grammars.md`
Blocked by: 

## Question

Which Rust crates give tree-sitter parsing for Python, Rust and C++ (crate names, current versions, the `tree-sitter` core version they pin, and whether they are maintained by the tree-sitter org or third parties)? What node types do each grammar emit for the constructs the function-scope rules need: function and method definitions (including closures/lambdas, nested defs, trait default methods, templates, constructors), the branch constructs that count for cyclomatic complexity (if/elif/else, loops, match arms, `?`, try/except/catch, ternaries, `and`/`or`/`&&`/`||`, switch/case, comprehension conditions), parameter lists, call expressions and member-access chains, and comments? How does the query API (`tree-sitter` S-expression queries) work from Rust for walking these, and how do the grammars report ERROR / MISSING nodes on partial parses? Note anything about C++ preprocessor directives and templates that tree-sitter-cpp documents as unsupported.

## Answer

Resolved 2026-10-26 by a research subagent. Full findings: `docs/research/tree-sitter-grammars.md` on branch `research/tree-sitter-grammars` (read with `git show research/tree-sitter-grammars:docs/research/tree-sitter-grammars.md`).

- **Crates:** `tree-sitter` 0.27.1 core, `tree-sitter-python` 0.25.0, `tree-sitter-rust` 0.24.2, `tree-sitter-cpp` 0.23.4, all from the tree-sitter GitHub org, MIT. Grammar crates pin no core version; they expose a `LanguageFn` via `tree-sitter-language ^0.1`, and compatibility is by ABI (core accepts 13–15; Python/Rust are 15, C++ is 14). All three load.
- **tree-sitter-cpp on crates.io is stale** (2024-11); master has C++20 modules but is unreleased. Node table verified against tagged releases. Superseded for the crate we build against: [File extensions, dialects and headers per language](08-language-surface.md) pins master `c009222808634c1014f82438d4883753516a2c24` instead of crates.io 0.23.4.
- **Every construct the function rules need has a named node type.** Gotchas: C++ constructors alias to `function_definition` with no `type:` field; Python `conditional_expression` has positional children, no fields; Rust `?` is `try_expression`; C++ `binary_expression` includes alternative tokens `and`/`or`; Python comprehension filters are `if_clause`; Rust match guards sit under `match_pattern` `condition:`.
- **Rust weak spot:** `macro_invocation` bodies are opaque `token_tree`s, so branches and calls inside `println!`/`assert!`/test macros are invisible.
- **C++ weak spots:** preprocessor conditionals parse as whole-item blocks at fixed nesting sites; a `#if` that splits a statement yields `ERROR` nodes (issue #308) and both branches appear in the tree. Macro bodies are one raw `preproc_arg` token; unknown macros in code misparse (issue #85). `#if` conditions alias to ordinary `binary_expression`/`call_expression`, so a `&&`/`||` counter must exclude ancestors of `preproc_if`/`preproc_elif`. The README documents nothing about the preprocessor or templates; template `<`/`>` ambiguity is open (issues #346, #318, #192).
- **Query API:** `Query::new(&Language, &str)` once per language; `QueryCursor::matches`/`captures` are streaming iterators; the Rust binding evaluates `eq?`/`match?`/`any-of?` itself and hands other predicates to the caller.
- **Partial parses always return a full tree** with `ERROR` nodes for bad spans and zero-width `MISSING` nodes; check with `is_error`/`is_missing`/`has_error`, and query `(ERROR)` and `(MISSING)` separately.
- **Unconfirmed:** whether Python `except*` parses in 0.25.0 (no `except_group_clause` node exists).
