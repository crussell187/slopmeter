# slopmeter

Slopmeter scores Python, Rust, and C++ functions and fails the run when a function the change touched is over a gate. The check is meant for an agent loop: the table names the function to edit, and the exit code is the verdict.

```
cargo install --git https://github.com/crussell187/slopmeter
```

From a checkout, `cargo install --path .`.

## Check a change

```
slopmeter
```

The default run walks the current directory. The diff is the merge-base with `main` through the working tree. If `main` is missing, the base is `master`, then `origin/main`, then `origin/master`. A function the diff touches has to come in under its gate, including one that improved and is still over the line. Findings in untouched functions are listed as inherited and do not fail the run. An empty diff passes. Outside git, the whole scored set is scored.

```
slopmeter --strict                 # score the whole tree
slopmeter --changed origin/main    # pick the base
slopmeter --json                   # print the JSON report
slopmeter path/to/tree             # score that directory
slopmeter explain F08              # one page on a rule
```

`--strict` and `--changed` together are a usage error, and so is `--changed` outside git.

The process exits 0 when the check passes, 1 when a gate fails, and 2 when it produces no check: a bad argument, an unknown rule, or a diff that cannot be computed. Warns are in the report and in the score. A warn does not fail the run.

`slop_score` is the sum of finding weights over the thousands of physical lines in the scored set. The same tree scores the same on a diff run and on `--strict`. The score does not fail the run.

## Rules

`slopmeter explain <rule>` prints how that rule counts and the threshold for each language.

| Rule | Measures | Level |
| --- | --- | --- |
| F01 | Cyclomatic complexity | gate |
| F02 | CRAP | gate |
| F03 | Cognitive complexity | warn, then gate |
| F04 | Parameter count | gate |
| F05 | Nesting | gate |
| F06 | Physical length | gate |
| F07 | Pass-through | warn |
| F08 | Member chain | gate |
| F09 | Inline I/O | warn |
| F10 | A void function mutates an argument | warn; Python and C++ |
| F11 | Logical lines | gate |
| F12 | A null literal returned or passed | warn; C++ |
| F15 | An empty or swallowing handler | gate |
| S06 | Coverage of a changed function | gate on a diff run |
| S07 | Mutation score of a changed function | gate on a diff run |

S06 reads an LCOV report. S07 reads [cargo-mutants](https://github.com/sourcefrog/cargo-mutants) `outcomes.json`, a [mutmut](https://github.com/boxed/mutmut) store, or a [mull](https://github.com/mull-project/mull) report. Slopmeter reads the report it is given and does not run the tests or the mutator. A missing report, or one older than a file the diff changed, fails each touched function on the default run. `--strict` keeps those rows from failing the run. Pass `--coverage` or `--mutation-rust`, `--mutation-python`, or `--mutation-cpp` to point at a report for one run.

## What gets scored

A run scores `.py`, `.rs`, and the C++ source, header, inline, and module extensions it walks: `.cpp`, `.cc`, `.cxx`, `.c++`, `.h`, `.hh`, `.hpp`, `.hxx`, `.inl`, `.ipp`, `.cppm`, and `.ixx`. Each file uses the profile for its extension.

Tests stay out until `--tests`. Gitignored files, a short list of build and vendor directories, and protobuf output stay out. The scored set is the directory the run starts in. A Cargo workspace, a Python monorepo, or a C++ tree with several targets is one set from the git root, and one member's subtree when the run starts inside that member.

## Config

A missing `slopmeter.toml` is the built-in profile. The file lives at the git root, and a nested file is ignored. It records departures: a coverage path, include and exclude patterns, a `warn` or a `gate`, and allow-list names. A count gate may rise to twice that language's built-in. An unknown key fails the run.

```toml
coverage = "coverage.lcov"
exclude = ["generated"]

[profile.python.F03]
gate = 12
```

## Waivers

A comment directly above a function excuses one gate up to the number written there. Python uses `#`. Rust and C++ use `//`.

```python
# slopmeter: waive F06 40 Parsing the header is one function.
```

The finding stays in the report. The verdict ignores it while the measurement is over the gate and at most that number. A waiver line the diff adds does not hold on that run. A line for a warn fails the run, because a warn is not a gate.
