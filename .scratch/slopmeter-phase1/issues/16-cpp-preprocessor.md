# C++ preprocessor and template handling

Type: grilling
Status: open
Blocked by: 01

## Question

tree-sitter-cpp parses source without running the preprocessor. Given what the grammar research found: how does the C++ adapter treat `#if/#ifdef` blocks (score all branches, the first, none?), function-like macros that hide branches or whole function bodies, and templates (score the definition once; ignore instantiations)? Does the adapter read `compile_commands.json` for defines and include paths, or stay purely syntactic in Phase 1 and report an `unparsed_ratio` per file so the reader knows how much to trust the numbers?
