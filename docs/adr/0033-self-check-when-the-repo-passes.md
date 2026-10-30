---
status: accepted
date: 2026-10-29
---

# The self-check joins CI when this repo passes

The thresholds exist, and `slopmeter --strict` on this repo still exits 1. Measured 2026-10-29, with `corpus/` excluded: 364 gate findings, 357 in `src/` and 7 in `prototype/ir-schema/`. A count gate may rise only to twice the built-in, so Rust cyclomatic complexity tops out at 20 while `Classifier::walk` is 59. F08 has no gate key, and `git` in `src/scope.rs` is a builder chain 11 deep against 1. A departure cannot make that run pass. The self-check stays off the fmt, clippy, and test job until `slopmeter --strict` on this repo exits 0. Bringing the crate under the gates stays part of this destination. There is no baseline of those findings. The joining run is the tool's own exit code.

## Considered options

- **Join now and fail the build.** Rejected. Every push and pull request would fail on code already in the tree. CI would stop meaning that a change broke the build.
- **Raise gates in `slopmeter.toml` until the run passes.** Rejected. The ceiling cannot reach cyclomatic complexity 59, and the hop limits are not a config key.
- **A baseline of the 364 findings.** Rejected. The check joins when this crate is under the gates. Today's findings are not grandfathered.

## Consequences

- [One crate, a public repo, and notes on main](0014-one-crate-public-repo.md) no longer joins the self-check as soon as the thresholds exist. The nine-repo validation run stays off every push.
- `exclude = ["corpus"]` stays. Include, exclude, and coverage are not departures.
- The self-check is `cargo run --quiet -- --strict` on the fmt, clippy, and test job. The nine-repo validation run stays off every push.
