# Shape of slopmeter.toml

Type: prototype
Status: open
Blocked by: 07, 08, 09

## Question

Write a complete example `slopmeter.toml` for a mixed Python+Rust repo: `[profile.python]`, `[profile.rust]`, `[profile.cpp]` tables with per-rule gate and warn thresholds, enable/disable per rule, allow-lists for F08, source include/exclude globs and test detection from the source-selection decision, `[override."path/glob"]` per-directory tightening, and coverage report locations. Decide precedence (CLI flag > override > profile > built-in default), what happens with no config file, and how the file validates (unknown keys fail?).
