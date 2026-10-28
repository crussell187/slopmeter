# Report unparsed_ratio per file and compute it from leaf error spans

Type: task
Status: resolved
Blocked by: 

## Question

The adapters compute `unparsed_ratio` but nothing in `src/check.rs` or `src/reporters.rs` reads it, so the per-file note ADR 0012 and 0017 promise never appears (review finding 7). It also saturates: fmt's `format.h` reports 0.993 while 317 functions score, because `collect_errors` at `src/adapters/cpp.rs:1523` counts the span of a wrapping top-level ERROR caused by one `#if FMT_HAS_INCLUDE(...) &&` line. Emit the note, and count only lines whose tokens sit directly under an ERROR or MISSING node (skip an ERROR whose children are well-formed items) in all three adapters. Done when the fmt headers report a ratio near 0.01 and the review's split-`#if` fixture reports a non-zero ratio.

## Answer

A scored file whose ratio is above 0 gets a note such as `The parser left 0.069 of the lines unscored.`, and the note does not change the verdict. A file at 0 is omitted. The ratio counts a line when a token sits directly under an `ERROR` or `MISSING` node. A recovered item (a named node that itself has children) does not contribute its span, so one wrapping error no longer marks the rest of the file. On the pinned fmt tree, `include/fmt/format.h` is 0.069, down from 0.993; `color.h` is 0.134 and `ranges.h` is 0.097. The lines that remain are tokens the grammar left under an error, mostly macro-heavy signatures, not the wrapper's span. The review's split-`#if` fixture (`p_split_cond`) reports 0.25. [ADR 0012](../../../docs/adr/0012-cpp-scored-as-text.md) now states this count.
