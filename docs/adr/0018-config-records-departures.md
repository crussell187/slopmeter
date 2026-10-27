---
status: accepted
date: 2026-10-27
---

# The config file records departures from the built-in profiles

A file that copied every threshold would pin numbers the validation run has not calibrated, and a key that turns a rule off would pass a clean repo without tuning. `slopmeter.toml` records only what this repo changes: a coverage path, include and exclude patterns, a `warn` or a `gate` on a rule that ships with that level, and member names added to an allow-list. The tool ships one profile per language, including the pass-through ratio and the two chain depths. A missing file is that profile and the default scored set.

## Considered options

- **A coverage path and allow-list names, and nothing else.** Rejected. A repo that needs a different gate would wait on a release of slopmeter. The profile is the table a project is allowed to change.
- **A full overlay: every rule's warn, gate, weight, and an enabled flag, plus a per-directory table.** Rejected. Turning a gate off is the move the validation run forbids. A path-dependent threshold makes the number on a finding a surprise. The corpus tunes one profile per language.

## Consequences

- The file is `slopmeter.toml` at the git root. Outside a git repo it is the working directory. A run from a subdirectory still reads that file. A second file lower in the tree is ignored.
- `--coverage <path>` replaces the file's `coverage` key for that run. It is the only flag that shares a key with the file. The flag may be absolute and may sit under an ignored directory. The file's value is repo-relative: an absolute path or a `..` segment fails the run. A path that is set and missing on disk is the coverage note, and CRAP stays uncomputed.
- `--tests` brings tests back for one run. `--strict`, `--changed`, `--json`, and `explain` have no file key.
- `include`, when set, keeps only matching files. `exclude` removes more. Patterns are anchored at the repo root. `*` stays inside one segment, `**` crosses segments, and a directory pattern covers everything under it, so `build` drops `build/lib.rs` and leaves `build.rs`. There is no `!` form. An absolute pattern or a `..` segment fails the run. Patterns match the path as written.
- An include cannot bring back a gitignored file, a denylisted directory, or a protobuf name. Tests stay out unless `--tests` is set.
- `allow` adds names to that language's adapter list. It does not replace the list. Each entry is one member name: an empty string, or a name containing `.`, `:`, or `/`, fails the run.
- F07 has no key. F08's only key is `allow`. A `warn` must sit strictly below the `gate` after the merge. A negative number fails the run. An omitted key stays the built-in.
- An unknown key, a value of the wrong type, invalid TOML, and an empty override fail before scoring. Empty means `include = []`, `exclude = []`, `allow = []`, `coverage = ""`, and a profile table with neither `warn` nor `gate`.
- The sample is `prototype/config-shape/`. Its numbers are one repo's overrides, not the built-in defaults.
