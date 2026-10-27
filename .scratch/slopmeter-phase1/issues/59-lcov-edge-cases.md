# Fix the LCOV join edge cases

Type: task
Status: open
Blocked by: 

## Question

`src/inputs/lcov.rs` unions two `SF` records that both suffix-match one scored file and the tie branch at `:256-262` is unreachable, so an all-zero and an all-hit record for the same file become coverage 1.0 with no note; `DA:1,1,` with an empty checksum aborts the run at `:145`; a `--coverage` path that is missing on disk prints "No coverage report was given" (`:71-76`) (review finding 16). Make two matching records a tie that leaves coverage absent with the ADR 0011 note, tolerate the empty checksum, and name the missing path in its note.
