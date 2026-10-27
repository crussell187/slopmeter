# Slopmeter Phase 1 review

Reviewed 2026-10-27 against the map (`.scratch/slopmeter-phase1/map.md`), the 23 ADRs, the design spec, and the binary built from `main` at `97d8911`. Four parallel reviews fed this: an adversarial C++ pass, an adversarial Python/Rust pass, a core-pipeline pass with fixture repos, and a re-run of the nine pinned corpus repos. Raw artifacts (per-function CSV, JSON reports, fixtures) sit under `/private/tmp/claude-501/-Users-christopherrussell-Documents-coding-projects-slopmeter/457423ad-50ef-4363-ab37-4b2476d7dfe3/scratchpad/` in `corpus-audit/`, `cpp-review/`, `py-rs-review/`, `core-review/`.

## Verdict

The build is real and well-made in the places the ADRs cover: 46 commits, 14k lines, 94% self line-coverage, meaningful unit tests, and Python numbers that match radon everywhere the ADRs don't deliberately differ. But as an **agent gate** it does not work yet, for three reasons that compound:

1. **The gates are vacuous.** Every per-language gate was set to the single worst function in three "clean" repos, so by construction no existing function fails. The sloppy corpus was then written to land at gate + 1, so it proves nothing. The outliers that set the gates were mostly code the tool should not have scored (a `#[cfg(test)]` module, a fuzz harness, a 600-entry constant table).
2. **Several whole-body blind spots** let an agent hide complexity at zero cost: Rust macro bodies, C++ member-init lists and function-try-blocks, macro-wrapped blocks, and the dead receiver-chain gate in Rust.
3. **Coverage is optional and mutation testing is absent.** F02 (CRAP) emits nothing when no LCOV report is supplied, there is no coverage-fraction rule at all, and mutation testing was a stated v1 non-goal that the build honoured.

Everything below is fixable, and most of it is small. The calibration method is the one decision that needs redoing rather than patching.

## What was built

A single Rust crate: source selection → three tree-sitter adapters → in-memory IR → analyzers → rule engine → text/JSON reporters, with `--strict`, `--changed=<ref>`, `--coverage`, `--tests`, `explain <rule>`, comment waivers, and a `slopmeter.toml` for departures. Nine function-scope rules: F01 CC, F02 CRAP, F03 cognitive, F04 params, F05 nesting, F06 physical length, F07 pass-through (warn), F08 member chain, F11 logical lines. Clean-repo and sloppy-corpus pins per language.

## Findings, ranked

### Tier 0: wrong verdicts

| # | Finding | Evidence | Where |
|---|---|---|---|
| 1 | **Gates flag nothing.** Shipped gates equal the corpus max (CC 48/94/136, length 196/581/713). At the spec's defaults, 1–12% of functions in each clean repo would fail, and the worst offenders are genuinely dense (httpx `urlparse` CC 48, serde `Container::from_ast` CC 94, re2 `Regexp::Parse` CC 79). | `corpus-audit/report.md`, `functions.csv` (10,492 fns) | `src/inputs/config.rs:150-198`, ADR 0013/0022 |
| 2 | **The scored set leaks test and generated code**, and those files set the gates. `#[cfg(test)]` modules are scored (513 of 5,791 Rust fns; `tests::parse_set_class` set F06 = 581). CLI11 `fuzz/` set C++ F01/F03/F11. re2 `testing/` (295 fns) is scored because only `test`/`tests` segments are excluded. fmt's dragonbox table set F06 = 713. 188 `.py` files inside C++ repos were scored. | Verified: an 11-assert `#[test]` fn in `src/lib.rs` is scored. | `src/inputs/select.rs:143-162`, `src/adapters/rust.rs:103` |
| 3 | **Silent false pass under `diff.noprefix` / `diff.mnemonicPrefix`.** The diff header parser expects `a/`/`b/`; with either git setting no function is ever "touched", so every finding is inherited and the run passes. | Verified: a 96-path function on a branch → `pass` with the finding under `inherited`. | `src/scope.rs:265-280, 420-431` |
| 4 | **Rust receiver-chain gate has never fired.** `peel` roots a chain only on an `identifier`; tree-sitter's `self` is a `self` node, so `self.a.b.c.d.e.f` is bucketed as "other" (limit 11). The 11-hop "other" limit was calibrated on these mis-bucketed chains. | Verified: `self.b.c.d.e.f` → `pass`. | `src/adapters/rust.rs:1136` |
| 5 | **Rust macro bodies are invisible** to F01/F03/F05/F08. `matches!(a, 1\|2\|…\|12 if a>0)` costs nothing; `dbg!(if … {…})` hides a whole body. Only `assert*` names are counted. | Fixtures `py-rs-review/fixtures/rs2` | `src/adapters/rust.rs:863, 122` |
| 6 | **C++ bodies that vanish without an ERROR node:** a constructor with a function-try-block is not a function at all; member-init lists are never walked (a lambda IIFE there holds an unscored body); a function-like macro wrapping a block (`SECTION(x) {…}`, `BOOST_FOREACH`) is parsed as a nested K&R function and its branches leave the parent. | Fixtures `cpp-review/fixtures/{05,15,16,17}` | `src/adapters/cpp.rs:232, 719, 68/145` |
| 7 | **`unparsed_ratio` never reaches the report**, and it saturates: fmt's `format.h` reports 0.993 while 317 functions scored, because one wrapping top-level ERROR (from `#if FMT_HAS_INCLUDE(...) &&`) counts every line. The one trust signal ADR 0012 relies on is both missing and wrong. | fmt run in `cpp-review/` | `src/adapters/cpp.rs:1513, 1523`; `check.rs`/`reporters.rs` never read it |
| 8 | **A deletion anywhere in a Rust or C++ file touches every function in the file** (false fail). A stale "adapters are later sessions" comment left `load_old` Python-only; the test at `scope.rs:1340` pins the bug as intended. | `core-review/b` | `src/scope.rs:166, 198-202` |
| 9 | **Running from a subdirectory** (or on a subdirectory path) reports "not a git repository", scores strictly, ignores root `slopmeter.toml` and `.gitignore`. A clone with only `origin/main` exits 2. | `core-review/a`, `c` | `src/scope.rs:225-257, 489-492` |
| 10 | **Config loophole and config gap.** An agent can set `[profile.rust.F03] gate = 4000000000` and nothing in the report records it. Conversely the file accepts only F03 levels and F08 `allow`, while ADR 0018/0022 describe per-rule gate overrides; F01/F04/F05/F06/F11 are compile-time constants. Setting F03 `warn` alone resets the gate to the generic 15. | `core-review/g` | `src/inputs/config.rs:299, 335-336, 385-393` |

### Tier 1: wrong numbers

| # | Finding | Where |
|---|---|---|
| 11 | `slop_score` is always 0.0: every weight is hard-coded 0, so weight-sorting is a no-op and the spec's "reduce slop_score below 5" loop cannot exist. | `src/rules.rs:283, 309` |
| 12 | Rust F11 is inflated 2–3×: `if`/`match`/closure bodies count as statements even in expression position, and the 138 gate was calibrated on the inflated count. | `src/adapters/rust.rs:650, 806` |
| 13 | F05 nesting counts boolean operators, lambdas and ternary operands as levels (`1 if a else 2 if b else 3` → 3). | `python.rs:990`, `rust.rs:906`, `analyzers.rs:77` |
| 14 | Python: a bare call is a chain hop, so `return make().a.b.c` gates at depth 4 while the Rust adapter drops the same root. Loop/try `else` adds +1 to CC against ADR 0003's text. | `python.rs:1218, 731-779` |
| 15 | F07 is trivially dodged (any second statement, or an `Ok(...)` wrapper) and over-fires (`return sorted(xs, key=…)` is 100%). `std::forward<Args>(args)...` is not recognised as forwarding. | `python.rs:1027`, `rust.rs:940`, `cpp.rs:1251` |
| 16 | Coverage join unions two `SF` records that both suffix-match one file; the ADR's "tie" branch is unreachable. `DA:1,1,` (empty checksum) aborts the run. A missing `--coverage` path is reported as "no report given". | `src/inputs/lcov.rs:145, 234-270` |
| 17 | Lambdas in Python defaults/decorator args and namespace/class-scope C++ lambdas (`std::function`, `static inline auto`) are unscored; empty `;` counts as a C++ statement. | `python.rs:527`, `cpp.rs:174-218, 821` |

### Tier 2: spec drift and polish

- **Rule renumbering and a missing rule.** Spec: F04 LLOC, F05 params, F06 nesting, F11 commented-out code. Build: F04 params, F05 nesting, F06 physical length, F11 logical lines. The commented-out-code rule has no implementation, no explain page, and no ADR recording its removal. Weights, warn bands (all but F03), and the F07 gate were dropped; exit codes were remapped to 0/1/2.
- **Explain pages** open with the generic stand-in ("gate at 8") then list contradictory per-language numbers; F03's page has a garbled sentence. The F02 "after" example deletes behaviour to reduce CC, which teaches exactly the wrong move.
- **CLI:** `--help`, `-h`, `--version` are "unknown argument"; `--changed` is silently ignored under `--strict`; a rejected bare waiver emits `"rule": ""`.
- **Self-check** `--strict` on this repo fails on the committed sloppy corpora (26 gates) and on `tests::git` in `scope.rs`; CI cannot adopt it yet.
- No GitHub remote exists despite ADR 0014.

## Mutation testing and 100% coverage

**Current state.** Not present, and not promised: the spec listed mutation testing as a v1 non-goal, scheduled S07 (mutation score on changed functions) for Phase 3 and S06 (coverage of changed lines) for Phase 2, and the map put both out of scope. The only coverage-aware rule is F02, which needs an LCOV file the user supplies; with no file it emits nothing and leaves a note, so an agent that never produces coverage is never gated on it. There is no rule on coverage fraction.

**What enforcing it would take.** Two new system-scope rules, diff-scoped like everything else:

- **S06, coverage of changed lines.** Already computable from the LCOV join the tool has: hit `DA` lines over `DA` lines inside touched functions' spans. Gate at 100% if that is the policy (the spec said 80/90). Make a missing report a *gate failure* in the default run, not a note, or the rule is optional. Branch coverage (`BRDA`) is currently ignored; 100% line coverage with 0% branch coverage is easy to reach, so read `BRDA` too.
- **S07, mutation score on changed functions.** Read a report, as with coverage, and compute `CRAP′ = CC² × (1 − mutation)³ + CC`. The per-language inputs differ: cargo-mutants writes `mutants.out/outcomes.json` with caught/missed/timeout/unviable per mutant and supports `--in-diff`, which matches diff scoping directly; mutmut 3 keeps results in a `mutants/` directory with an `export-cicd-stats` command and no documented per-mutant JSON, so the adapter would read its store or wrap the CLI; mull (C++) emits a mutation-testing-elements JSON report and SARIF but needs a compilation database, and dextool likewise. **That last point collides with ADR 0012:** C++ is scored as text precisely to avoid `compile_commands.json`, but any C++ mutation gate requires a build. Either the C++ mutation rule accepts a compile database as its own input, or C++ gets S06 only.
- **Gaming defenses the spec already named** and that matter more once coverage is a gate: S05 assertion density (a test that runs everything and asserts nothing reaches 100% coverage), and the escalation check for relocated complexity.

A 100% line-coverage gate on changed lines is a one-session change on top of the existing LCOV join. The mutation rule is a Phase 2 effort: one report reader per language, the C++ build-input decision, and a validation run with the sloppy corpora's tests.

## Next steps, in order

**Blockers before any agent uses the gate**
1. Recalibrate. Fix the scored set first (honour `#[cfg(test)]` and `#[test]`, exclude `fuzz/` and `testing/`, don't score `.py` under C++ repos, let generated tables be waived), re-pin, then set gates at a stated percentile of the production-only set (p95 for CC/cognitive/lloc/lines, p97.5 for params/nesting) and record the percentile in ADR 0022. The audit's evidence-backed defaults: CC 12, cognitive warn 10 / gate 15, params 4–6, nesting 4, lines 60, lloc 25–30, CRAP 8, chains 2/1. Rewrite the sloppy corpora to be sloppy, not gate + 1.
2. Pin the diff shape (`--src-prefix=a/ --dst-prefix=b/` or `-c diff.noprefix=false`) and add a test with repo-local config. Resolve the root with `git rev-parse --show-toplevel`; fall back to `origin/main`.
3. Call all three adapters in `load_old` and invert the test at `scope.rs:1340`.
4. Root Rust chains on the `self` node kind, then recalibrate F08.
5. Surface `unparsed_ratio` as the per-file note the ADRs promise, and compute it from leaf ERROR/MISSING spans so a header with one bad `#if` line is not 0.99.
6. Give every rule a weight (spec: gate 10, warn 1–3) or remove `slop_score` until it means something.

**C++ fidelity** (your concern, and warranted)
7. Walk `field_initializer_list`; accept a `try_statement` sibling when `body` is absent; fold a nested "function" whose declarator has no types back into its parent. Those three close the body-hiding routes at about fifty lines.
8. Decide macros explicitly: today a function-like macro can hide any number of branches with no signal. Options are an opt-in `compile_commands.json` preprocessing mode for repos that have one, a per-file macro-density note, or an F-rule that flags a touched function whose body contains an unexpanded function-like macro invocation. The text-only decision in ADR 0012 is defensible for headers, but it needs a visible signal to be honest.
9. Descend into Rust macro token trees for `matches!`, `dbg!`, `format!`/`println!`, `vec!`, `assert*`.

**Coverage and mutation**
10. Add S06 (coverage of changed lines, `BRDA` included) with a missing report failing the default run. One session.
11. Record the mutation-testing decision on the map as a Phase 2 effort with the C++ build-input question as its first ticket; then one report reader per language starting with cargo-mutants.

**Spec reconciliation and polish**
12. Record the rule renumbering and the dropped commented-out-code rule in an ADR, or implement F11 as specced and move logical lines to F04.
13. Fix the explain pages (one threshold table, no generic stand-in line, an F02 "after" that adds a test instead of deleting a branch), `--help`/`--version`, the config `warn`-only reset, the lcov edge cases, and add a config-departure note to the report.
14. Add `slopmeter.toml` excludes for `corpus/` so the self-check can join CI, and create the GitHub remote ADR 0014 calls for.
