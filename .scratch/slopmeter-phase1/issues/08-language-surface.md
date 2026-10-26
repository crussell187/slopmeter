# File extensions, dialects and headers per language

Type: grilling
Status: resolved
Blocked by: 

## Question

Which files does each adapter claim? Python: `.py` only, or also `.pyi` stubs and `.pyx`? Rust: `.rs`, with `build.rs` and `#[cfg(test)]` modules treated how? C++: `.cpp/.cc/.cxx/.h/.hpp/.hxx`, and does the adapter also claim plain C (`.c`, C-style `.h`)? Are functions defined in headers scored where the header lives, and can a header's functions be double-counted when included from many translation units? Which language standard versions must parse (Python 3.9+?, Rust 2021 edition, C++17/20)?

## Answer

Resolved 2026-10-26 with Christopher. Why the C++ grammar pin moved off crates.io: [ADR 0004](../../../docs/adr/0004-tree-sitter-cpp-master-pin.md).

Each adapter claims files by extension and parses each file once. It does not follow `#include`. A function defined in a header is scored in that header. A declaration there is not a function. The out-of-line definition is scored in the file that holds the body.

Claimed extensions:

- Python: `.py`
- Rust: `.rs`
- C++: `.cpp`, `.cc`, `.cxx`, `.c++`, `.h`, `.hh`, `.hpp`, `.hxx`, `.inl`, `.ipp`, `.cppm`, `.ixx`

Left out: `.pyi`, `.pyx`, `.c`, `.ccm`, `.mxx`, `.cxxm`. A `.h` is parsed as C++. `build.rs` is claimed because it is `.rs`. Whether `build.rs` and `#[cfg(test)]` modules are in the scored set stays on [Source selection and test-file policy](09-source-selection.md).

The promise is Python 3.9 and later, Rust as tree-sitter-rust 0.24.2 parses it, and C++17 plus C++20 module syntax. `match` is in tree-sitter-python 0.25.0. `except*` is not: that grammar has no node for it. There is no Rust edition switch. tree-sitter-cpp is git revision `c009222808634c1014f82438d4883753516a2c24` (master on 2026-09-17), which has `module_declaration` and `import_declaration` at ABI 15. crates.io 0.23.4 does not. Switch back to crates.io when a release contains those nodes. Parsing `export module` and `import` does not resolve imports or read built module files.

What the tool does with a file the grammar cannot parse stays in the fog until [C++ preprocessor and template handling](16-cpp-preprocessor.md).
