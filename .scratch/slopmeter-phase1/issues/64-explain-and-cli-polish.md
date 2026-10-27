# Fix the explain pages and the CLI surface

Type: task
Status: open
Blocked by: 46, 63

## Question

Every explain page opens with the generic stand-in ("gate at 8") then lists different per-language numbers, F03's page has a garbled threshold sentence, and F02's after example deletes a branch to lower CC, which teaches the wrong move (review, Tier 2). `--help`, `-h` and `--version` are unknown arguments (`src/cli.rs:8-11`), `--changed` is silently ignored under `--strict` and outside git, and a rejected bare waiver emits `"rule": ""` (`src/rules.rs:297`). After recalibration and the numbering decision: one threshold table per page with no stand-in line, an F02 after that adds a test, help and version output, a usage error for conflicting flags, and a named rule on the bare-waiver row.
