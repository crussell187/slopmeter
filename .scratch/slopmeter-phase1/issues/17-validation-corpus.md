# Validation corpus per language

Type: grilling
Status: resolved
Blocked by: 08

## Question

Which clean open-source repos form the validation corpus for each of Python, Rust and C++ (how many — the spec said five for TS; three per language may be enough), chosen for what (size range, test suites that emit coverage, reputation for design quality)? Your own Python projects (auto_midi, bow_arc_extraction, roomverb, solo_string_livener) are candidates for the Python side. How is the deliberately sloppy agent-written repo produced per language — prompt an agent with no quality constraints against a seeded task, or hand-write fixtures that violate each rule? What is the pass criterion: clean repos produce zero gate findings, sloppy repos trip every rule they should?

## Answer

Resolved 2026-10-26 with Christopher. Why the run tunes on nine pinned repos: [ADR 0013](../../../docs/adr/0013-validation-corpus.md). The glossary terms are **Validation run** and **Sloppy repo** in `CONTEXT.md`.

The corpus is scored with `--strict`. A fresh checkout has an empty diff, and an empty diff gates nothing.

Thresholds move until every clean repo has zero gate findings. Warns may remain, including F07. A gate may loosen. It may not be switched off to force a pass. At those same thresholds the sloppy repo must gate on F01–F06, F08, and F11, and it must warn on F07. If the only way to trip a rule on the sloppy repo is to fail a clean repo too, that rule comes back for a decision. The clean repo stays.

Each language has three public repos. Each has tests that can emit LCOV, and each is small enough to score in one session. The commit recorded for the run is the corpus. The default branch is not.

- Python: `psf/requests`, `encode/httpx`, `pallets/click`
- Rust: `serde-rs/serde`, `rust-lang/regex`, `tokio-rs/bytes`
- C++: `fmtlib/fmt`, `google/re2`, `CLIUtils/CLI11`

Clap stays out, because its builder chains would teach F08 to allow long call runs. nlohmann/json stays out, because one huge header would pull every gate loose. spdlog stays out, because it vendors fmt. The author's Python projects (`auto_midi`, `bow_arc_extraction`, `roomverb`, `solo_string_livener`) stay out of the bar the thresholds move toward. They can smoke-test the tool.

The sloppy repo is one pinned tree per language. An agent writes it from a fixed seed that describes one small program and asks for the bad structures in plain language: a function that decides too much, one that only forwards, a chain through several objects, too many parameters, and a complex function the tests never reach. The seed does not name rule ids or thresholds and gives no quality instructions. The tests can emit LCOV. The tree is kept when every gating rule fires and F07 warns. If a rule does not fire, the seed is revised and the agent runs again. Later validation runs use the pin. Hand-written fixtures that prove one rule in isolation stay with the test suite. They are not this corpus.
