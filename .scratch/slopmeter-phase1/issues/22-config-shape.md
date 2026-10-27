# Shape of slopmeter.toml

Type: prototype
Status: resolved
Blocked by: 07, 08, 09

## Question

Write a complete example `slopmeter.toml` for a mixed Python+Rust repo: `[profile.python]`, `[profile.rust]`, `[profile.cpp]` tables with per-rule gate and warn thresholds, enable/disable per rule, allow-lists for F08, source include/exclude globs and test detection from the source-selection decision, `[override."path/glob"]` per-directory tightening, and coverage report locations. Decide precedence (CLI flag > override > profile > built-in default), what happens with no config file, and how the file validates (unknown keys fail?).

## Answer

Resolved 2026-10-27 with Christopher. Why the file records departures and why a rule cannot be switched off: [ADR 0018](../../../docs/adr/0018-config-records-departures.md). The sample is [prototype/config-shape/](../../../prototype/config-shape/).

`slopmeter.toml` sits at the git root, or in the working directory outside a git repo. A run from a subdirectory still reads that file. A second file lower in the tree is ignored. A missing file is the built-in profiles and the default scored set.

The file may set `coverage`, `include`, `exclude`, and, under `[profile.<language>.<rule>]`, `warn`, `gate`, or `allow`. A present key replaces that key. An omitted language, rule, or level stays on the built-in profile. There is no off switch, no weight, no score ceiling, no per-directory table, and no test-detection glob.

`coverage` is one repo-relative LCOV path. An absolute path or a `..` segment fails the run. `--coverage <path>` replaces it for that run and may be absolute, including under an ignored directory such as `target/`. When neither is set, or the path is not on disk, the run keeps the coverage note and leaves CRAP uncomputed. `--tests` brings tests back for one run. `--strict`, `--changed`, `--json`, and `explain` have no file key.

`include`, when set, keeps only matching files. `exclude` removes more. Patterns are anchored at the repo root, match the path as written, and use `*` inside one segment and `**` across segments. A directory pattern covers everything under it. There is no `!` form. An absolute pattern or a `..` segment fails the run. An include cannot bring back a gitignored file, a denylisted directory, or a protobuf name.

`allow` adds member names to that language's adapter list and does not replace it. Each entry is one member name. An empty string, or a name containing `.`, `:`, or `/`, fails the run. F07 has no key. F08's only key is `allow`. The pass-through ratio and the two chain depths stay in the shipped profile.

After the merge, a two-level rule keeps its warn strictly below its gate. A negative number, a level the rule does not ship, a value of the wrong type, an unknown key, and invalid TOML fail before scoring. So do empty overrides: `include = []`, `exclude = []`, `allow = []`, `coverage = ""`, and a profile table with neither `warn` nor `gate`.
