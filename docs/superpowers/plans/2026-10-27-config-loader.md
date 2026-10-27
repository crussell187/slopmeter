# Config Loader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `inputs::load_config` reads `slopmeter.toml` as departures from the built-in profile.

**Architecture:** `src/inputs/config.rs` parses the file with the `toml` crate and validates it before scoring. `Config::retains` applies include, then exclude. Source selection still drops gitignore, denylist directories, protobuf names, and tests. The caller passes the git root; this function reads only `{root}/slopmeter.toml`.

**Tech Stack:** Rust stable, edition 2021, `toml` 1.1.8.

## Global Constraints

- A missing file is the built-in profile. F03 warns above 10 and gates above 15 in every language, from the shape in [ADR 0006](../../docs/adr/0006-gate-on-the-finding.md). F07 stays at 80 with no key. F08 stays at 2 hops from the receiver and 1 otherwise; its only key is `allow`.
- Until [Which metric each rule id measures](../../.scratch/slopmeter-phase1/issues/34-rule-ids.md), the loader accepts `warn` and `gate` on `F03` only, plus `allow` on `F08`.
- `coverage` is repo-relative. An absolute path, a `..` segment, or an empty string fails. A path that is not on disk still loads. `set_coverage_override` may be absolute.
- `include` and `exclude` are anchored at the root, case-sensitive, with `*` inside one segment and `**` across segments. A directory pattern covers its contents, so `build` drops `build/lib.rs` and leaves `build.rs`. There is no `!` form.
- An unknown key, a wrong type, invalid TOML, a negative number, an empty override, and a warn that is not strictly below the gate after the merge fail before scoring.

---

### Task 1: Load the file

**Files:**
- Modify: `Cargo.toml` (`toml` = `1.1.8`)
- Create: `src/inputs/mod.rs`
- Create: `src/inputs/select.rs` (moved from `src/inputs.rs`)
- Create: `src/inputs/config.rs`
- Test: `src/inputs/config.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::Language`
- Produces: `slopmeter::inputs::{load_config, Config, ConfigError, LanguageProfile}`

- [x] **Step 1: Write the failing test**

`load_config` is not defined, so the built-in-profile test fails to compile.

- [x] **Step 2: Parse departures and reject the rest**

Implemented in `src/inputs/config.rs`.

- [x] **Step 3: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/inputs
git commit -m "Load slopmeter.toml departures from the built-in profile."
```
