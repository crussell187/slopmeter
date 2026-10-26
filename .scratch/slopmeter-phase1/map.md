# Slopmeter Phase 1 — map

Label: wayfinder:map
Spec: [Clean Software Enforcement Tool — Design Spec](https://claude.ai/artifact/9KjxgFjszCmYaC8nWhPprJ)
Charted: 2026-10-26

## Destination

A shipped Phase 1 MVP of **slopmeter**: a Rust binary that scores Python, Rust and C++ source against the function-scope rules (F01–F08, F11), with F07 a warn and the rest gates, reads coverage for CRAP, emits the JSON report and the worst-first text table with exit codes, supports `--changed` and `explain`, and is installable with `cargo install` from this repo. Shipped means the validation run passes for each of the three languages: a set of clean open-source repos pass and a deliberately sloppy agent-written repo fails on every gating rule it should, with thresholds tuned from that run.

## Notes

- **Execution is in scope.** This map carries building, not only deciding (overrides the wayfinder plan-don't-do default). Decision tickets come first; build slices graduate from the fog once the IR, report and config are frozen.
- **Standing decisions from charting:** name is `slopmeter` (binary, config file `slopmeter.toml`, cache dir, crate); core language is Rust (see `docs/adr/0001-rust-core.md`); adapters are built in the order Python → Rust → C++, and the IR freezes after the Python adapter proves it on real repos. TypeScript, which the spec put first, is deferred (Out of scope).
- **Skills to consult:** `/grilling` and `/domain-modeling` for decision tickets (glossary lives in `CONTEXT.md`); `/prototype` and `/design-an-interface` for prototype tickets; `/research` for research tickets; `/writing-plans` when build slices graduate.
- **Tracker:** local markdown (this directory). No GitHub CLI on this machine; migrate the map if a GitHub repo is created (see the repo-layout ticket).
- **Spec gaps found while charting:** the spec's JSON report schema and escalation example code blocks are empty in the doc. The report schema is therefore a prototype ticket, not a given.
- **Spec terms the tool keeps:** the rule IDs (F01…), tiers (A/B/C), gate vs warn, `slop_score` per KLOC, `fix_shape`, `anti_fix`, `principle`, `scope: changed` with `inherited` findings, the six-stage pipeline (inputs → adapters → IR → analyzers → rule engine → reporters).

## Decisions so far

<!-- one line per closed ticket: gist, then the link for the detail -->
- [tree-sitter grammars for Python, Rust and C++ from Rust](issues/01-tree-sitter-grammars.md) — `tree-sitter` 0.27.1 with the org's python 0.25.0 / rust 0.24.2 crates; crates.io cpp 0.23.4 has no modules, so the language-surface decision pins master instead; Rust macro bodies are opaque and C++ `#if`/macros split statements and leak into `&&`/`||` counts, so [C++ preprocessor and template handling](issues/16-cpp-preprocessor.md) has its facts.
- [How radon, rust-code-analysis and lizard count cyclomatic complexity](issues/04-cross-check-tools.md) — all agree on base 1 + if/loop/catch/ternary/&&/||; they split on `with`, `assert`, `finally`, Python `match`, Rust match arms (+3 vs +2), C++ `default` and `#if`, so ticket 07 must pick per-language parity targets or a bounded delta.
- [SonarSource cognitive complexity rules mapped to Python, Rust, C++](issues/03-cognitive-complexity.md) — implement from white paper v1.7 Appendix B; sonar-python and the official sonar-rust give per-construct tables; clippy's lint is a different metric; C++ reference is the community sonar-cxx; recursion is unimplemented everywhere and `match` handling differs per language.
- [Coverage formats: coverage.py LCOV, cargo-llvm-cov, gcov/lcov](issues/02-coverage-formats.md) — no tool gives a per-function coverage fraction via FN/FNDA, so derive it from DA line hits intersected with the IR function's line range; never key on FN names (three naming schemes, two mangled); parse both legacy FN/FNDA and lcov 2.2's FNL/FNA.
- [Install the Rust toolchain on this machine](issues/05-install-rust-toolchain.md) — stable rustc/cargo 1.99.1 (`aarch64-apple-darwin`) via rustup 1.29.2, plus rustfmt, clippy, llvm-tools-preview (LLVM 23.1.1) and cargo-llvm-cov 0.9.1; `~/.cargo/bin` is on PATH through `~/.zshenv`.
- [What counts as a function, per language](issues/06-what-is-a-function.md) — a named definition with a written body; a lambda inside a function folds into it, and a lambda bound to a name outside any function is itself a function; identity is the file of the body plus the qualified name, with C++ parameter types for overloads ([ADR 0002](../../docs/adr/0002-what-counts-as-a-function.md)).
- [Cyclomatic complexity counting rules, per language](issues/07-cc-counting-rules.md) — own rules: base 1, +1 per decision point; a residual arm, a jump, and syntax that does not choose a path add nothing; the cross-check tools may differ by a listed delta ([ADR 0003](../../docs/adr/0003-cyclomatic-complexity-rules.md)).
- [File extensions, dialects and headers per language](issues/08-language-surface.md) — claim `.py`, `.rs`, and the C++ source, header, inline, and module extensions (`.cppm`, `.ixx`); parse each file once and do not follow `#include`; promise Python 3.9+, Rust as the pinned grammar parses it, and C++17 plus C++20 modules via tree-sitter-cpp `c009222` ([ADR 0004](../../docs/adr/0004-tree-sitter-cpp-master-pin.md)).
- [Source selection and test-file policy](issues/09-source-selection.md) — every claimed file in the repo, minus gitignore, build and vendor directories, protobuf names, and tests; tests return only through a switch; `build.rs`, `examples/`, and `benches/` stay in ([ADR 0005](../../docs/adr/0005-scored-set.md)).
- [Gate on the max finding or on slop_score](issues/10-gate-max-vs-aggregate.md) — the run fails on a Tier A finding over its gate level, not on `slop_score`; the score is reported and has no ceiling in Phase 1; a lower level on the same rule is a warn ([ADR 0006](../../docs/adr/0006-gate-on-the-finding.md)).
- [Diff-scoped by default, inherited findings, and --strict](issues/11-diff-scoped-default.md) — default run gates only touched functions; a touched function still over its gate level fails even if the value improved; untouched findings are inherited and do not gate; an empty diff gates nothing; `--strict` scores the whole repo ([ADR 0007](../../docs/adr/0007-diff-scoped-default.md)).
- [What --changed means](issues/12-changed-semantics.md) — the diff is the merge-base with `main` through the working tree, or `master` if `main` is missing; a function is touched when a changed line hits its span, including decorators and attributes; no base ref fails the run; outside git the whole scored set is scored ([ADR 0008](../../docs/adr/0008-what-changed-means.md)).
- [Pass-through and Demeter detection without symbol resolution](issues/13-passthrough-demeter-exemptions.md) — F07 is a syntactic pass-through at 80% or more of the ordinary parameters and warns; F08 gates a member chain over two hops from the receiver or one hop otherwise, with a per-language allow-list and a trailing `build` run ignored; the two-implementation exemption is dropped ([ADR 0009](../../docs/adr/0009-passthrough-warns-chain-gates.md)).
- [What a Phase 1 finding carries](issues/14-finding-payload.md) — one finding per rule per function, at the worst site; it carries the level it is over (`gate` or `warn`), a fix shape filled with the measured value and that threshold, and one anti-fix sentence; it does not predict the edit ([ADR 0010](../../docs/adr/0010-finding-payload.md)).

## Not yet specified

- **Build slices.** Once the IR schema, adapter contract, report schema and config shape are frozen, the implementation gets sliced (likely via `/writing-plans`) into task tickets: core + Python adapter + LCOV reader first, ending in the Python validation run; then the Rust adapter and its run; then the C++ adapter and its run. Each slice is execution, in scope per Notes.
- **Threshold calibration.** The spec's defaults (CC > 10, CRAP > 8.0, 40 LLOC, 4 params…) are for typed class-based languages and come from Martin's tools, not from LLM-written code. Each language profile gets its own defaults after its validation run. Can't be sharpened until a corpus has been scored.
- **Parse failures.** What the tool does with a file tree-sitter cannot fully parse (syntax error, unsupported construct, C++ macro soup): skip, score partial, or fail the run. Depends on the C++ preprocessor ticket.
- **Multi-package repos.** Cargo workspaces, Python monorepos, C++ projects with several targets: how source selection, `--changed` and the KLOC denominator treat them.
- **C++ build-system awareness.** Whether the C++ adapter reads `compile_commands.json` for include paths and defines, or stays purely syntactic. Depends on the preprocessor ticket.
- **Testing slopmeter itself.** Fixture files per rule, golden-file tests for the reporters, and cross-checks of CC against radon, rust-code-analysis and lizard. Shape depends on the crate layout and the cross-check research.
- **`explain` content.** Authoring the principle, counting method, examples, and the longer anti-fix list for each of the nine shipped rules. The finding already carries one sentence of each. Execution; waits on the explain prototype and on the rules that are still open.
- **IR cache between runs.** The spec caches the serialized IR so analyzers can be written in any language and so Phase 3 can compute deltas. Whether Phase 1 writes the cache at all is unsettled.

## Out of scope

- **TypeScript / JavaScript adapter.** The spec's first language; deferred in favour of Python, Rust, C++ (charting decision). Returns as its own effort.
- **Go, Java/Kotlin, C# adapters.** Spec rollout order 4–6.
- **Module and system scope (Phase 2).** Import graph, cycles, Ce/Ca, interface width, leverage, pass-through ratio, LCOM4, clone detection, hotspots, SARIF.
- **Gaming defenses and the loop (Phase 3).** Call depth S03, assertion density S05, `previous_slop_score` deltas, escalation, MCP server, pre-commit hook.
- **Architecture scope (A01–A10) and the Phase 3/4 additions**, mutation testing, LLM-judge pass for Tier C.
- **The agent acceptance test** (with-tool vs without-tool run scored by an independent ruler). Not part of the done line.
- **Public distribution** beyond `cargo install` from the repo: crates.io, Homebrew, GitHub Action, npm wrapper.
