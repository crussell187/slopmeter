# Slopmeter Phase 1 — map

Label: wayfinder:map
Spec: [Clean Software Enforcement Tool — Design Spec](https://claude.ai/artifact/9KjxgFjszCmYaC8nWhPprJ)
Charted: 2026-10-26

## Destination

A shipped Phase 1 MVP of **slopmeter**: a Rust binary that scores Python, Rust and C++ source against the function-scope gates (F01–F08, F11), reads coverage for CRAP, emits the JSON report and the worst-first text table with exit codes, supports `--changed` and `explain`, and is installable with `cargo install` from this repo. Shipped means the validation run passes for each of the three languages: a set of clean open-source repos pass and a deliberately sloppy agent-written repo fails on every rule it should, with thresholds tuned from that run.

## Notes

- **Execution is in scope.** This map carries building, not only deciding (overrides the wayfinder plan-don't-do default). Decision tickets come first; build slices graduate from the fog once the IR, report and config are frozen.
- **Standing decisions from charting:** name is `slopmeter` (binary, config file `slopmeter.toml`, cache dir, crate); core language is Rust (see `docs/adr/0001-rust-core.md`); adapters are built in the order Python → Rust → C++, and the IR freezes after the Python adapter proves it on real repos. TypeScript, which the spec put first, is deferred (Out of scope).
- **Skills to consult:** `/grilling` and `/domain-modeling` for decision tickets (glossary lives in `CONTEXT.md`); `/prototype` and `/design-an-interface` for prototype tickets; `/research` for research tickets; `/writing-plans` when build slices graduate.
- **Tracker:** local markdown (this directory). No GitHub CLI on this machine; migrate the map if a GitHub repo is created (see the repo-layout ticket).
- **Spec gaps found while charting:** the spec's JSON report schema and escalation example code blocks are empty in the doc. The report schema is therefore a prototype ticket, not a given.
- **Spec terms the tool keeps:** the rule IDs (F01…), tiers (A/B/C), gate vs warn, `slop_score` per KLOC, `fix_shape`, `anti_fix`, `principle`, `scope: changed` with `inherited` findings, the six-stage pipeline (inputs → adapters → IR → analyzers → rule engine → reporters).

## Decisions so far

<!-- one line per closed ticket: gist, then the link for the detail -->
- [tree-sitter grammars for Python, Rust and C++ from Rust](issues/01-tree-sitter-grammars.md) — use `tree-sitter` 0.27.1 with the org's python 0.25.0 / rust 0.24.2 / cpp 0.23.4 crates; all needed constructs have node types; Rust macro bodies are opaque and C++ `#if`/macros split statements and leak into `&&`/`||` counts, so ticket 16 has its facts.
- [How radon, rust-code-analysis and lizard count cyclomatic complexity](issues/04-cross-check-tools.md) — all agree on base 1 + if/loop/catch/ternary/&&/||; they split on `with`, `assert`, `finally`, Python `match`, Rust match arms (+3 vs +2), C++ `default` and `#if`, so ticket 07 must pick per-language parity targets or a bounded delta.
- [SonarSource cognitive complexity rules mapped to Python, Rust, C++](issues/03-cognitive-complexity.md) — implement from white paper v1.7 Appendix B; sonar-python and the official sonar-rust give per-construct tables; clippy's lint is a different metric; C++ reference is the community sonar-cxx; recursion is unimplemented everywhere and `match` handling differs per language.

## Not yet specified

- **Build slices.** Once the IR schema, adapter contract, report schema and config shape are frozen, the implementation gets sliced (likely via `/writing-plans`) into task tickets: core + Python adapter + LCOV reader first, ending in the Python validation run; then the Rust adapter and its run; then the C++ adapter and its run. Each slice is execution, in scope per Notes.
- **Threshold calibration.** The spec's defaults (CC > 10, CRAP > 8.0, 40 LLOC, 4 params…) are for typed class-based languages and come from Martin's tools, not from LLM-written code. Each language profile gets its own defaults after its validation run. Can't be sharpened until a corpus has been scored.
- **Parse failures.** What the tool does with a file tree-sitter cannot fully parse (syntax error, unsupported construct, C++ macro soup): skip, score partial, or fail the run. Depends on the C++ preprocessor ticket.
- **Multi-package repos.** Cargo workspaces, Python monorepos, C++ projects with several targets: how source selection, `--changed` and the KLOC denominator treat them.
- **C++ build-system awareness.** Whether the C++ adapter reads `compile_commands.json` for include paths and defines, or stays purely syntactic. Depends on the preprocessor ticket.
- **Testing slopmeter itself.** Fixture files per rule, golden-file tests for the reporters, and cross-checks of CC against radon, rust-code-analysis and lizard. Shape depends on the crate layout and the cross-check research.
- **`explain` content.** Authoring the principle, counting method, examples and anti-fix list for each of the nine shipped rules. Execution; follows the finding-payload decision.
- **IR cache between runs.** The spec caches the serialized IR so analyzers can be written in any language and so Phase 3 can compute deltas. Whether Phase 1 writes the cache at all is unsettled.

## Out of scope

- **TypeScript / JavaScript adapter.** The spec's first language; deferred in favour of Python, Rust, C++ (charting decision). Returns as its own effort.
- **Go, Java/Kotlin, C# adapters.** Spec rollout order 4–6.
- **Module and system scope (Phase 2).** Import graph, cycles, Ce/Ca, interface width, leverage, pass-through ratio, LCOM4, clone detection, hotspots, SARIF.
- **Gaming defenses and the loop (Phase 3).** Call depth S03, assertion density S05, `previous_slop_score` deltas, escalation, MCP server, pre-commit hook.
- **Architecture scope (A01–A10) and the Phase 3/4 additions**, mutation testing, LLM-judge pass for Tier C.
- **The agent acceptance test** (with-tool vs without-tool run scored by an independent ruler). Not part of the done line.
- **Public distribution** beyond `cargo install` from the repo: crates.io, Homebrew, GitHub Action, npm wrapper.
