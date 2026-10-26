# The adapter interface contract

Type: prototype
Status: open
Blocked by: 19

## Question

Design the seam every language adapter fills: inputs (selected files, per-language config, coverage map?) and the single output (IR). Use /design-an-interface to compare at least two shapes — e.g. one trait with `parse(file) -> Module` per file vs a whole-project `analyze(files) -> IR` that can see across files — against the three adapters' needs (C++ headers seen from many translation units, Rust module tree, Python packages). How do adapters report unparsed regions and unknown edges? The contract is what makes adding a language touch one adapter and nothing else.
