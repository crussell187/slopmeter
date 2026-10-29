---
status: accepted
date: 2026-10-26
---

# The validation run tunes on nine pinned repos

The bar below, that a clean repo has no gate findings and that a threshold loosens until it passes, is replaced by [Count gates are a percentile of the pinned clean repos](0031-count-gates-are-a-percentile.md). The nine repos and their commits still stand. A `--strict` run of a clean repo shows the functions over the gate. The sloppy repo still gates on F01–F06, F08, and F11, and warns on F07. A gate may not be switched off.

The clean set is three public repos per language, each with tests that can emit LCOV, each small enough to score in one session, each pinned to the commit recorded for the run. Python: `psf/requests`, `encode/httpx`, `pallets/click`. Rust: `serde-rs/serde`, `rust-lang/regex`, `tokio-rs/bytes`. C++: `fmtlib/fmt`, `google/re2`, `CLIUtils/CLI11`. The sloppy repo is one pinned tree per language, written by an agent from a fixed seed that asks for the bad structures in plain language and does not name rule ids. It is kept only once every gating rule fires, and later runs reuse the pin.

## Considered options

- **Hand-write a fixture per rule and call that the corpus.** Rejected. Those fixtures belong to the test suite. The corpus is a program an agent wrote, pinned so the run does not move.
- **Tune against the author's own Python projects.** Rejected. The thresholds would learn one person's style. Those projects can smoke-test the tool. They are not the bar.
- **Include clap, nlohmann/json, or spdlog.** Rejected. Clap's builders would teach F08 to allow long call chains. nlohmann/json is one huge header. spdlog vendors fmt.

## Consequences

- A fresh checkout is scored with `--strict`. An empty diff gates nothing, so the default run cannot be the validation run.
- The default branch of a clean repo is not the corpus. The recorded commit is.
