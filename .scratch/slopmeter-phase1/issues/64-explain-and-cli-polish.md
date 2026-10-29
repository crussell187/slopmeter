# Fix the explain pages and the CLI surface

Type: task
Status: resolved
Blocked by: 46, 63

## Question

Every explain page opens with the generic stand-in ("gate at 8") then lists different per-language numbers, F03's page has a garbled threshold sentence, and F02's after example deletes a branch to lower CC, which teaches the wrong move (review, Tier 2). `--help`, `-h` and `--version` are unknown arguments (`src/cli.rs:8-11`), `--changed` is silently ignored under `--strict` and outside git, and a rejected bare waiver emits `"rule": ""` (`src/rules.rs:297`). After recalibration and the numbering decision: one threshold table per page with no stand-in line, an F02 after that adds a test, help and version output, a usage error for conflicting flags, and a named rule on the bare-waiver row.

## Answer

Each explain page's threshold section is one table, a row per language, and it does not say "Shipped stand-in". F03 no longer says that 8 and 12 pass every language. F02's after keeps the branches and adds `test_classify`. `--help` and `-h` print the usage and exit 0. `--version` prints `slopmeter` and the package version and exits 0. `--changed` with `--strict` exits 2 with `--changed and --strict cannot be combined`. `--changed` outside a git repository exits 2 with `--changed needs a git repository`. A waiver line that names no rule is a finding whose rule is `waiver`. The consequences are on [The text table is the check a person reads](../../../docs/adr/0019-text-table-and-explain.md), [The change is the branch plus the working tree](../../../docs/adr/0008-what-changed-means.md), and [A waiver excuses one gate up to a stated measurement](../../../docs/adr/0023-a-waiver-excuses-one-gate.md).
