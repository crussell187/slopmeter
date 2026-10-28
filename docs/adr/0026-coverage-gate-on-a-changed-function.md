---
status: accepted
date: 2026-10-28
---

# A changed function is covered when every executable line is hit

F02 emits nothing when coverage is absent, so an agent that never produces a report is never gated on coverage. S06 is that gate. It reads the same fraction CRAP already uses: hit `DA` lines over `DA` lines in the body span, nested functions excluded. The gate is 1. A fraction below 1 fails, and 1 passes. There is no warn. Branch records stay out, because the three report producers emit them only when branch data is turned on. The finding weighs 10. It sits on the first missed line, or on the function line when no fraction was produced. The cell reads `0.5 < 1`, or `absent < 1`.

A touched function fails when its fraction is below 1, when its file has no matching record, or when the records tie. A matched span with no `DA` lines stays off the gate. An untouched function with a computed fraction below 1 is inherited. An untouched function whose fraction was never produced gets no row. An empty diff still passes. A missing report fails the default run once a function is touched, and the note stays.

`--strict` does not fail on coverage. A missing report stays a note and adds no S06 row. Every S06 row from a present report stays inherited, so a diff run and a `--strict` run score the same tree when a report is present. A run outside git is scored in full and uses that same placement. A waiver line that names S06 fails the run. The anti-fix is a test that runs the function and asserts nothing.

## Considered options

- **Gate only the executable lines the diff changed.** Rejected. A one-line edit inside an uncovered function would pass as soon as that line was hit.
- **Warn at 80 and gate at 90.** Rejected. The other deterministic gates have no warn band, and a warn would let uncovered lines through.
- **Read `BRDA`.** Rejected. A gate that ignores missing branch records is optional, and a gate that requires them fails the default report from each tool.
- **Let a comment waive S06.** Rejected. The agent writes the comment and would skip the gate.
- **Fail `--strict` at 100%.** Rejected. The clean repos are not fully covered, and that run would fail on nearly every function with an uncovered line.
- **Store 0 when the fraction was not produced.** Rejected. That tells the agent the lines were seen and missed.

## Consequences

- The explain page is `slopmeter explain S06`. F09 stays the unknown-rule id.
- [Coverage is a line fraction joined by file and range](0011-coverage-by-line-range.md) still computes the fraction. [A finding's weight is fixed for its rule and level](0025-finding-weight-is-fixed.md) includes S06 at 10. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md) does not excuse this one.
