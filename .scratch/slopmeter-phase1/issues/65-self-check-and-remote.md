# Put the self-check in CI and create the GitHub remote

Type: task
Status: open
Blocked by: 46, 47, 66

## Question

`slopmeter --strict` on this repo fails on the committed sloppy corpora (26 gates) and on inline test helpers, so ADR 0014's self-check step cannot join CI, and no GitHub remote exists because `gh` was not installed (review, Tier 2). After the scored-set fix, the recalibration and the corpus rewrite: add a `slopmeter.toml` excluding `corpus/`, add the self-check to `.github/workflows/ci.yml`, install the GitHub CLI, create the public repo on Chris's account, and push `main` and the research branches.
