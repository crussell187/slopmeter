# File extensions, dialects and headers per language

Type: grilling
Status: open
Blocked by: 

## Question

Which files does each adapter claim? Python: `.py` only, or also `.pyi` stubs and `.pyx`? Rust: `.rs`, with `build.rs` and `#[cfg(test)]` modules treated how? C++: `.cpp/.cc/.cxx/.h/.hpp/.hxx`, and does the adapter also claim plain C (`.c`, C-style `.h`)? Are functions defined in headers scored where the header lives, and can a header's functions be double-counted when included from many translation units? Which language standard versions must parse (Python 3.9+?, Rust 2021 edition, C++17/20)?
