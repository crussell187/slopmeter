---
status: accepted
date: 2026-10-26
---

# One crate, a public repo, and notes on main

Phase 1 is one crate, `slopmeter`. The pipeline stages are modules, and each language adapter is a module. The tree-sitter grammars are dependencies of that crate, including the pinned C++ revision. A workspace would have frozen a crate boundary before the IR schema and the adapter contract exist.

The GitHub repo is public, on the personal account, so `cargo install --git` has a URL. The map stays the markdown in `.scratch/`, and the ADRs stay in `docs/adr/`. The research notes live in `docs/research/` on `main`. Tickets do not move to GitHub Issues.

CI on push to `main` and on pull requests is `cargo fmt --check`, `clippy` with warnings denied, and `cargo test`, on stable. `slopmeter --strict` on this repo joins that job when that command exits 0 ([The self-check joins CI when this repo passes](0033-self-check-when-the-repo-passes.md)). The nine-repo validation run is not part of every push.

## Considered options

- **A crate per pipeline stage.** Rejected. Nothing in Phase 1 is a separate library, and the seams those crates would lock are still open.
- **Private repo, or GitHub Issues as the tracker.** Rejected. A private repo blocks install for anyone else. The decisions are already files in git, and a second tracker would fork the map.
- **Leave the research notes on their branches.** Rejected. A clone of `main` would not contain the evidence the tickets cite.

## Consequences

- The remote is the public repo https://github.com/crussell187/slopmeter.
- The self-check is `cargo run --quiet -- --strict` on that job. The corpus run stays off that job.
