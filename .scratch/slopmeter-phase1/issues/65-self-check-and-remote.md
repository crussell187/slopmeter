# Put the self-check in CI and create the GitHub remote

Type: task
Status: open
Blocked by: 46, 47, 66, 77

## Question

`slopmeter --strict` on this repo fails on the committed sloppy corpora (26 gates) and on inline test helpers, so ADR 0014's self-check step cannot join CI, and no GitHub remote exists because `gh` was not installed (review, Tier 2). After the scored-set fix, the recalibration and the corpus rewrite: add a `slopmeter.toml` excluding `corpus/`, add the self-check to `.github/workflows/ci.yml`, install the GitHub CLI, create the public repo on Chris's account, and push `main` and the research branches.

## Comments

The exclude is in `slopmeter.toml`. A strict run of `corpus/python/sloppy` from this repo exits 0 with no findings, and the corpus tests copy the tree out so they still score it. A strict run of the repo on 2026-10-29 still exits 1: 364 gate findings, 357 in `src/` and 7 in `prototype/ir-schema/`. The CI step stays off until [Whether the self-check joins CI while the crate is over its gates](77-self-check-over-gates.md) is resolved. The public repo is https://github.com/crussell187/slopmeter. `main` and the four `research/*` branches are pushed. `gh` is logged in as crussell187.
