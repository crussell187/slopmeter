---
status: accepted
date: 2026-10-26
---

# The default scored set leaves tests out

The gate is for the code under review, and a test is often long and branchy on purpose. `assert` already adds 1 to cyclomatic complexity, so the same gates would fail clean repos on their tests. A second profile for tests would be another set of thresholds, and those numbers wait on a corpus. The default scored set is every claimed file in the repo. It is not limited to `src/`. Tests are out until a switch turns them on. Gitignore, a short list of dependency and build directories, and well-known protobuf names stay out even when an include glob names them.

## Considered options

- **Score tests with the same gates.** Rejected. Table-driven tests and `assert` would dominate the validation run.
- **Score tests under a separate profile.** Rejected for Phase 1. The production thresholds are not calibrated yet, and a second table would double that problem.
- **Default to `src/` only.** Rejected. Python and C++ trees do not share that layout, so most of a repo would be invisible.

## Consequences

- An include of everything does not score tests. The switch does.
- `#[cfg(test)]` modules inside an otherwise scored file are out, helpers included.
- `build.rs`, `examples/`, and `benches/` stay in.
- `include` and `exclude` are the file keys. `--tests` is the switch. [The config file records departures](0018-config-records-departures.md).
