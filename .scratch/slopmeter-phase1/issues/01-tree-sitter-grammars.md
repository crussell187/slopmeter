# tree-sitter grammars for Python, Rust and C++ from Rust

Type: research
Status: open
Blocked by: 

## Question

Which Rust crates give tree-sitter parsing for Python, Rust and C++ (crate names, current versions, the `tree-sitter` core version they pin, and whether they are maintained by the tree-sitter org or third parties)? What node types do each grammar emit for the constructs the function-scope rules need: function and method definitions (including closures/lambdas, nested defs, trait default methods, templates, constructors), the branch constructs that count for cyclomatic complexity (if/elif/else, loops, match arms, `?`, try/except/catch, ternaries, `and`/`or`/`&&`/`||`, switch/case, comprehension conditions), parameter lists, call expressions and member-access chains, and comments? How does the query API (`tree-sitter` S-expression queries) work from Rust for walking these, and how do the grammars report ERROR / MISSING nodes on partial parses? Note anything about C++ preprocessor directives and templates that tree-sitter-cpp documents as unsupported.
