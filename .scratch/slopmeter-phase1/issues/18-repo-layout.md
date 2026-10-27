# Repo, crate layout, CI and where the GitHub repo lives

Type: grilling
Status: resolved
Blocked by: 

## Question

This directory is now a git repo with no remote. Does slopmeter get a GitHub repo (which account, public or private), and does the map migrate from local markdown to GitHub Issues then (needs the `gh` CLI installed)? Is the Rust project a single crate with modules per pipeline stage, or a Cargo workspace with one crate per stage (inputs, adapters, ir, analyzers, rules, reporters) plus the CLI? Where do language grammars live (one adapter crate per language)? What CI runs on push: fmt, clippy, tests, and eventually the self-check? Where do research notes and ADRs live (currently `docs/research/` and `docs/adr/`)?

## Answer

Resolved 2026-10-26 with Christopher. Why Phase 1 is one crate in a public repo, with the notes on `main`: [ADR 0014](../../../docs/adr/0014-one-crate-public-repo.md).

Phase 1 is one crate, `slopmeter`. The stages are modules: inputs, adapters, IR, analyzers, rules, reporters. Python, Rust, and C++ are modules under the adapters. The tree-sitter grammars are dependencies of this crate. The C++ grammar stays the pinned git revision. `cargo install` from this repo installs that binary.

The GitHub repo is public, on Christopher's personal account. The map stays in `.scratch/`. The ADRs stay in `docs/adr/`. Creating the remote does not move the tickets onto GitHub Issues. `gh` is not installed, so this decision does not create the remote. That waits until the CLI is available.

The four research notes are on `main` in `docs/research/`: `cognitive-complexity.md`, `coverage-formats.md`, `cross-check-tools.md`, and `tree-sitter-grammars.md`. The `research/*` branches can remain. They are not the copy a clone of `main` stands on.

GitHub Actions on push to `main` and on pull requests runs `cargo fmt --check`, `clippy` with warnings denied, and `cargo test`, on stable. `slopmeter --strict` on this repo joins that job after the validation run has tuned the thresholds. The nine-repo validation run is not part of every push.
