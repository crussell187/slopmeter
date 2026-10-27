# Report unparsed_ratio per file and compute it from leaf error spans

Type: task
Status: open
Blocked by: 

## Question

The adapters compute `unparsed_ratio` but nothing in `src/check.rs` or `src/reporters.rs` reads it, so the per-file note ADR 0012 and 0017 promise never appears (review finding 7). It also saturates: fmt's `format.h` reports 0.993 while 317 functions score, because `collect_errors` at `src/adapters/cpp.rs:1523` counts the span of a wrapping top-level ERROR caused by one `#if FMT_HAS_INCLUDE(...) &&` line. Emit the note, and count only lines whose tokens sit directly under an ERROR or MISSING node (skip an ERROR whose children are well-formed items) in all three adapters. Done when the fmt headers report a ratio near 0.01 and the review's split-`#if` fixture reports a non-zero ratio.
