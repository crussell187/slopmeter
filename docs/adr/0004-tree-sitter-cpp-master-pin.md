---
status: accepted
date: 2026-10-26
---

# Pin tree-sitter-cpp to master until modules are released

C++20 module units are part of the language slopmeter promises to score. The crates.io crate `tree-sitter-cpp` 0.23.4 (2024-11-11) has no module syntax, so a module file would fail to parse and its functions would never be scored. The unpublished master branch has `module_declaration` and `import_declaration` at ABI 15, which `tree-sitter` 0.27.1 accepts. We pin commit `c009222808634c1014f82438d4883753516a2c24` (2026-09-17, "Explicitly inherit highlights from C") and switch back to crates.io when a release contains those nodes. Parsing `export module` and `import` does not resolve imports or read built module files.

## Considered options

- **Stay on crates.io 0.23.4 and leave modules out of the promise.** Rejected. It would make module units invisible, which is the same hole as not claiming a header.

## Consequences

- The C++ adapter depends on that git revision, not on the 0.23.4 version named in the grammar research.
- `.cppm` and `.ixx` are claimed extensions, parsed once with the same grammar as other C++ files.
- The node table in the grammar research was checked against the 0.23.4 tag. Module nodes come from this commit, not from that table.
