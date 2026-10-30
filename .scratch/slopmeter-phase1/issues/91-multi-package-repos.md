# Whether a multi-package repo is one scored set

Type: grilling
Status: resolved
Blocked by:

## Question

A Cargo workspace, a Python monorepo, and a C++ tree with several targets are one git repo. Today a run walks the working directory, reads `slopmeter.toml` from the git root and ignores a nested file, touches functions the diff hits inside that directory, and divides `slop_score` by the physical lines of the files it loaded. Decide whether a package boundary changes any of that. If the directory stays the scored set, package manifests are not read. If a package is its own set, say what a package is in each language, which one a run scores, and whether `--changed` and the KLOC denominator follow the package or the git repo.

## Answer

Resolved 2026-10-30 with Christopher. A package boundary changes none of the three. The scored set is the directory the run starts in, under the git root's `slopmeter.toml`. A workspace scored from its root is one set. A run started inside a member scores that subtree, and a sibling is not in the walk, the diff, or the KLOC denominator. [ADR 0005](../../../docs/adr/0005-scored-set.md). No code change: the walk, `--relative`, and the line count already do this.
