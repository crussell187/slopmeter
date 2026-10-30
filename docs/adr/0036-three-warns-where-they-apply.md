---
status: accepted
date: 2026-10-29
---

# F09, F10 and F12 ship as warns only where they apply

The spec's function catalog has three warn rules its phase plan never placed. Each is a good rule in some of this effort's languages and noise in the others, so each ships for the languages where its signal is clean and is marked not applicable elsewhere. All three are warns at weight 1, with no gate level. A waiver line that names one of them fails the run, as for F07.

- **F09, an I/O dependency constructed inline**, ships in Python, Rust and C++. A function that calls a name on that language's list of constructors and connect calls, matched by its written name without symbol resolution, gets one finding at the first such call. `slopmeter.toml` takes a per-language `allow` for F09 that adds names which do not count, in the shape of F08's `allow`, and each added name is a departure note.
- **F10, a void function that mutates an argument**, ships in Python and C++ and not in Rust. In Python a function with no `return` of a value that assigns to a parameter's attribute or index, uses an augmented assignment on one, or calls a known mutator on one gets the finding. In C++ a function returning `void` with a non-const reference or non-const pointer parameter gets it. In Rust `&mut` on a function returning `()` is the language's idiom.
- **F12, a null literal returned or passed**, ships in C++ only. A `return nullptr`, `return NULL`, or a `nullptr` or `NULL` argument gets the finding. Rust has no null. In Python `None` as an argument or a default is the language's own idiom.

## Considered options

- **Ship none of them in this effort.** Rejected. Each one has at least one language where the rule is a signature or syntax fact and the spec names the principle it guards.
- **Ship each in every language.** Rejected. F10 would flag every `&mut` method in Rust, and F12 would flag almost every Python file.
- **Give them a gate level.** Rejected. They are heuristics on written names and shapes. The spec keeps them warn-only, and a warn moves the score and not the verdict.
- **A config switch that turns one off per language.** Rejected. No rule has a switch. F09's `allow` is the one adjustment a project needs, because its own I/O wrapper is a legitimate site.

## Consequences

- `explain` gains three pages. A page says which languages the rule scores and that the others are not applicable.
- [A finding's weight is fixed](0025-finding-weight-is-fixed.md) gains three weight-1 warns.
- The name lists, the mutator list, and the sloppy-corpus additions are build-ticket detail: [Score an inline I/O dependency as F09](../../.scratch/slopmeter-phase1/issues/87-score-f09.md), [Score a void function that mutates an argument as F10](../../.scratch/slopmeter-phase1/issues/88-score-f10.md), and [Score a null literal as F12](../../.scratch/slopmeter-phase1/issues/89-score-f12.md).
