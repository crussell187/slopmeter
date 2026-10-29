---
status: accepted
date: 2026-10-29
---

# Count gates are a percentile of the pinned clean repos

Setting each gate to the worst function in the clean repos made the gate flag nothing. A count gate is the nearest-rank percentile of the scored set of that language's three pinned commits. Sort the values and take the one at rank `ceil(p / 100 × n)`. Cyclomatic complexity, the cognitive gate, function length, and logical lines use p95. Parameter count and nesting use p97.5. The cognitive warn uses p90, and when that integer is not below the gate it is one less. A parameter with a default counts, and so do `*args` and `**kwargs`. The member-chain limits are 2 hops from the receiver and 1 otherwise. CRAP stays at 8, the pass-through warn at 80, the coverage gate at 100%, and the mutation gate at 1.

The integers below were measured on 2026-10-29 from the commits already recorded in `corpus/*/pins.txt`. Python had 1,329 functions, Rust 5,610, and C++ 2,879. A `.py` file inside a C++ repo was in neither count. A later measurement may replace these integers. A pin bump on its own does not.

| Rule | Python | Rust | C++ |
|---|---:|---:|---:|
| Cyclomatic complexity | 10 | 10 | 12 |
| Cognitive warn | 8 | 3 | 7 |
| Cognitive gate | 12 | 6 | 15 |
| Parameter count | 10 | 3 | 4 |
| Nesting | 3 | 3 | 3 |
| Function length | 51 | 45 | 41 |
| Logical lines | 19 | 17 | 25 |

A value above the integer fails. The integer itself passes. A function in a clean repo that sits over the gate fails with the others. The waiver comment already defined is the only excuse, and the pins do not carry waivers.

## Considered options

- **Keep the worst function as the gate.** Rejected. That is the rule that made every clean repo pass, including the functions the review named as real slop.
- **Ignore a Python parameter that has a default.** Rejected. The count is the width of the function. A default would become the way to clear it, and Rust and C++ already count a defaulted parameter.
- **Waive the clean-repo tail in advance.** Rejected. A generated table and a wide `match` are gates. The comment sits on the function a project owns.
- **Round an interpolated percentile.** Rejected. Nearest rank is an observed measurement, and at most the named tail sits strictly over it.

## Consequences

- [The validation run tunes on nine pinned repos](0013-validation-corpus.md) no longer requires a clean repo to have no gate findings. A `--strict` run shows the tail. Thresholds are not loosened until that tail disappears.
- [Each function rule has one metric and a hard stand-in](0022-rule-ids-and-stand-in-thresholds.md) still names the metric. These integers replace the stand-ins and the per-language maxima.
- The config ceiling stays twice the built-in, so the ceiling moves with these integers. An omitted cognitive warn is that language's warn, not 10. [A count gate may rise to twice the built-in](0028-count-gate-twice-the-built-in.md).
