# Fix the LCOV join edge cases

Type: task
Status: resolved
Blocked by: 

## Question

`src/inputs/lcov.rs` unions two `SF` records that both suffix-match one scored file and the tie branch at `:256-262` is unreachable, so an all-zero and an all-hit record for the same file become coverage 1.0 with no note; `DA:1,1,` with an empty checksum aborts the run at `:145`; a `--coverage` path that is missing on disk prints "No coverage report was given" (`:71-76`) (review finding 16). Make two matching records a tie that leaves coverage absent with the ADR 0011 note, tolerate the empty checksum, and name the missing path in its note.

## Answer

Resolved 2026-10-28. A second `SF` record that matches a scored file already matched is a tie: coverage stays absent and the note is `Coverage records tie for this file.` `DA:1,1,` is a listed line. A report path that is not a file notes `No coverage report at <path>. CRAP was not computed.` No path at all keeps `No coverage report was given. CRAP was not computed.` The longer scored path still wins when one record matches two files. [ADR 0011](../../../docs/adr/0011-coverage-by-line-range.md).
