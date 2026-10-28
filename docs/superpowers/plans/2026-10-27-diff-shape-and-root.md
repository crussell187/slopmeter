# Diff Shape and Repo Root Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** A changed function stays touched when the repo sets `diff.noprefix` or `diff.mnemonicPrefix`, and a run from a subdirectory (or on a subdirectory path) diff-scopes against the repo, reads the root `slopmeter.toml`, and honors the root `.gitignore`.

**Architecture:** `git diff` forces `a/` and `b/` prefixes. `scope::repo_root` is `git rev-parse --show-toplevel` with no ceiling directory, so a subdirectory is still inside the work tree. `check::score` loads `slopmeter.toml` from that root, walks the requested directory with the root `.gitignore`, matches include and exclude against repo-relative paths, and partitions the requested directory. The base ref is local `main`, then `master`, then `origin/main`, then `origin/master`.

**Tech Stack:** Rust stable, git, `cargo test`.

## Global Constraints

- The diff is the merge-base with the base ref through the working tree (ADR 0008). `--changed=<ref>` still wins. A missing base still fails with `no base ref; pass --changed=<ref> or --strict`.
- Outside a git repository, the run scores the whole tree and notes `not a git repository; scoring the whole tree`.
- `slopmeter.toml` is read from the git root. A second file lower in the tree is ignored (ADR 0018). Include and exclude patterns stay anchored at that root.
- `.gitignore` at the git root applies to a subdirectory run. A directory that is not a git repo still does not apply a `.gitignore`.
- File paths in the report are relative to the git root. `--relative` stays on `git diff` so a diff taken from a subdirectory still lines up with paths relative to that directory.
- `git show` of a deleted line uses the work-tree prefix, so a deletion from a subdirectory touches only the function it hits.

---

### Task 1: Pin the diff header prefixes

**Files:**
- Modify: `src/scope.rs` (`diff_files`, tests)
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `partition(root, files, findings, ScopeOptions)`
- Produces: the same signature. Repo-local `diff.noprefix` and `diff.mnemonicPrefix` still yield touched functions.

- [x] **Step 1: Write the failing tests**

Add both tests next to `a_changed_line_gates_that_function_and_strict_gates_both`. Each builds a repo, sets one local diff option, edits one function, and asserts that function is the only finding.

```rust
#[test]
fn noprefix_still_touches_the_edited_function() {
    let root = repo("noprefix", "main");
    write(&root, "src/a.py", PAIR);
    commit_all(&root, "base");
    git(&root, &["config", "diff.noprefix", "true"]);
    write(&root, "src/a.py", &PAIR.replacen("return 1", "return 2", 1));

    let scoped = partition(
        &root,
        &[parse("src/a.py", &fs::read(root.join("src/a.py")).unwrap())],
        &[],
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    let bytes = fs::read(root.join("src/a.py")).unwrap();
    let file = parse("src/a.py", &bytes);
    let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);
    let scoped = partition(
        &root,
        &[file],
        &findings,
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(scoped.scope, Scope::Changed);
    assert!(scoped.findings.iter().any(|finding| finding.name == "touched"));
    assert!(scoped.findings.iter().all(|finding| finding.name == "touched"));
    assert!(scoped.inherited.iter().all(|finding| finding.name == "other"));
}

#[test]
fn mnemonic_prefix_still_touches_the_edited_function() {
    let root = repo("mnemonic", "main");
    write(&root, "src/a.py", PAIR);
    commit_all(&root, "base");
    git(&root, &["config", "diff.mnemonicPrefix", "true"]);
    write(&root, "src/a.py", &PAIR.replacen("return 1", "return 2", 1));
    let bytes = fs::read(root.join("src/a.py")).unwrap();
    let file = parse("src/a.py", &bytes);
    let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);

    let scoped = partition(
        &root,
        &[file],
        &findings,
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert!(scoped.findings.iter().any(|finding| finding.name == "touched"));
    assert!(scoped.inherited.iter().all(|finding| finding.name == "other"));
}
```

The first test in the block above is the shape to keep: one `partition` call, after the file is parsed. Drop the duplicate.

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib scope::tests::noprefix_still_touches_the_edited_function scope::tests::mnemonic_prefix_still_touches_the_edited_function -- --exact`

Expected: FAIL because no finding is touched (`findings` empty, both names inherited).

- [x] **Step 3: Force `a/` and `b/` on the diff**

In `diff_files`, pass config overrides before the subcommand and prefix flags after it:

```rust
"--no-pager",
"-c",
"core.quotePath=false",
"-c",
"diff.noprefix=false",
"-c",
"diff.mnemonicPrefix=false",
"diff",
"--src-prefix=a/",
"--dst-prefix=b/",
"--no-color",
"--no-ext-diff",
"--unified=0",
"--find-renames",
"--relative",
base,
```

- [x] **Step 4: Run the tests to verify they pass**

Run the same `cargo test` command. Expected: PASS.

### Task 2: Treat a subdirectory as the work tree

**Files:**
- Modify: `src/scope.rs` (`work_tree`, `git`, `load_old`, `untracked_paths`, tests)
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `partition`
- Produces: `pub fn repo_root(start: &Path) -> Result<Option<PathBuf>, ScopeError>`. `None` means not a work tree. `Some` is the canonical toplevel, including when `start` is below it.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn a_subdirectory_diff_touches_only_the_edited_function() {
    let root = repo("subdir-diff", "main");
    write(&root, "nested/src/a.py", PAIR);
    write(&root, "src/b.py", "def outside():\n    return 1\n");
    commit_all(&root, "base");
    write(
        &root,
        "nested/src/a.py",
        &PAIR.replacen("return 1", "return 2", 1),
    );
    write(&root, "src/b.py", "def outside():\n    return 2\n");
    let nested = root.join("nested");
    let bytes = fs::read(nested.join("src/a.py")).unwrap();
    let file = parse("src/a.py", &bytes);
    let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);

    let scoped = partition(
        &nested,
        &[file],
        &findings,
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(scoped.scope, Scope::Changed);
    assert!(scoped.notes.is_empty());
    assert!(scoped.findings.iter().any(|finding| finding.name == "touched"));
    assert!(scoped.findings.iter().all(|finding| finding.name == "touched"));
    assert!(scoped.inherited.iter().all(|finding| finding.name == "other"));
}

#[test]
fn a_subdirectory_deletion_touches_only_that_function() {
    let root = repo("subdir-delete", "main");
    let before = "\
def touched():
    value = 1
    return value

def other():
    return 0
";
    let after = "\
def touched():
    return value

def other():
    return 0
";
    write(&root, "nested/src/a.py", before);
    commit_all(&root, "base");
    write(&root, "nested/src/a.py", after);
    let nested = root.join("nested");
    let file = parse("src/a.py", &fs::read(nested.join("src/a.py")).unwrap());
    let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

    let scoped = partition(
        &nested,
        &[file],
        &findings,
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(names(&scoped.findings), vec!["touched"]);
    assert_eq!(names(&scoped.inherited), vec!["other"]);
}

#[test]
fn an_untracked_file_in_a_subdirectory_is_touched() {
    let root = repo("subdir-untracked", "main");
    write(&root, "nested/src/old.py", "def other():\n    return 0\n");
    commit_all(&root, "base");
    write(&root, "nested/src/new.py", "def touched():\n    return 1\n");
    let nested = root.join("nested");
    let files = vec![
        source_file("src/old.py", vec![function("other", 1, 2)]),
        source_file("src/new.py", vec![function("touched", 1, 2)]),
    ];
    let findings = vec![gate("src/old.py", "other"), gate("src/new.py", "touched")];

    let scoped = partition(
        &nested,
        &files,
        &findings,
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(names(&scoped.findings), vec!["touched"]);
    assert_eq!(names(&scoped.inherited), vec!["other"]);
}
```

Move `outside_git_the_whole_set_is_scored` onto a directory that has no parent repository (`std::env::temp_dir()`), so dropping the ceiling does not make `target/` look like this repo.

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib scope::tests::a_subdirectory_diff_touches_only_the_edited_function scope::tests::a_subdirectory_deletion_touches_only_that_function scope::tests::an_untracked_file_in_a_subdirectory_is_touched -- --exact`

Expected: FAIL. The run is strict and notes that it is not a git repository, or a deletion touches both functions.

- [x] **Step 3: Resolve the toplevel and keep subdirectory paths**

`work_tree` is `repo_root(start)?.is_some()`. `repo_root` runs `git rev-parse --show-toplevel` and canonicalizes the path. `git` no longer sets `GIT_CEILING_DIRECTORIES`; it removes that variable.

`git status --porcelain=v1 -z` paths stay relative to the toplevel. Strip `git rev-parse --show-prefix` (the trailing slash included) from those paths, and drop a path outside the prefix.

`load_old` asks `git show` for `{base}:{prefix}{old_path}` so a `--relative` diff from a subdirectory still finds the old file.

- [x] **Step 4: Run the tests to verify they pass**

Run the Task 2 command plus `scope::tests::outside_git_the_whole_set_is_scored`. Expected: PASS.

### Task 3: Fall back to origin/main, then origin/master

**Files:**
- Modify: `src/scope.rs` (`resolve_base`), `docs/adr/0008-what-changed-means.md`
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `rev_exists`, `git merge-base`
- Produces: the same `resolve_base` result, with two more candidates after the local branches.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn origin_main_is_the_base_when_no_local_branch_exists() {
    let root = repo("origin-main", "feature");
    let body = "def touched():\n    return 1\n";
    write(&root, "src/a.py", body);
    commit_all(&root, "base");
    git(&root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));

    let scoped = partition(
        &root,
        &[source_file("src/a.py", vec![function("touched", 1, 2)])],
        &[gate("src/a.py", "touched")],
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(names(&scoped.findings), vec!["touched"]);
}

#[test]
fn origin_master_is_the_base_when_origin_main_is_missing() {
    let root = repo("origin-master", "feature");
    let body = "def touched():\n    return 1\n";
    write(&root, "src/a.py", body);
    commit_all(&root, "base");
    git(&root, &["update-ref", "refs/remotes/origin/master", "HEAD"]);
    write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));

    let scoped = partition(
        &root,
        &[source_file("src/a.py", vec![function("touched", 1, 2)])],
        &[gate("src/a.py", "touched")],
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(names(&scoped.findings), vec!["touched"]);
}

#[test]
fn local_main_wins_over_origin_main() {
    let root = repo("local-wins", "main");
    let body = "def touched():\n    return 1\n";
    write(&root, "src/a.py", body);
    commit_all(&root, "base");
    git(&root, &["checkout", "-b", "feature"]);
    write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
    commit_all(&root, "edit");
    git(&root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    let scoped = partition(
        &root,
        &[source_file("src/a.py", vec![function("touched", 1, 2)])],
        &[gate("src/a.py", "touched")],
        ScopeOptions {
            changed: None,
            strict: false,
        },
    )
    .unwrap();

    assert_eq!(names(&scoped.findings), vec!["touched"]);
}
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib scope::tests::origin_main_is_the_base_when_no_local_branch_exists scope::tests::origin_master_is_the_base_when_origin_main_is_missing scope::tests::local_main_wins_over_origin_main -- --exact`

Expected: FAIL with `no base ref; pass --changed=<ref> or --strict` for the origin tests. `local_main_wins_over_origin_main` already passes; keep it so the candidate order stays locked.

- [x] **Step 3: Extend the candidate list**

```rust
for name in ["main", "master", "origin/main", "origin/master"] {
    if !rev_exists(root, name)? {
        continue;
    }
    return merge_base(root, name);
}
Err(ScopeError::NoBase)
```

`merge_base` is today's `git merge-base HEAD <name>` success path. A branch that exists but has no merge-base still returns `NoBase` without trying the next name.

Update ADR 0008: if neither local `main` nor `master` exists, the base is `origin/main`, then `origin/master`.

- [x] **Step 4: Run the tests to verify they pass**

Expected: PASS. `no_main_or_master_fails_and_asks_for_a_ref_or_strict` still passes.

### Task 4: Read the root config and gitignore from a subdirectory

**Files:**
- Modify: `src/check.rs`, `tests/check_command.rs`
- Test: `tests/check_command.rs`

**Interfaces:**
- Consumes: `scope::repo_root`, `load_config`, `select`, `partition`
- Produces: the same `score(root, options)`. `root` may be a subdirectory. Config, gitignore, coverage from the file, and report paths use the git root. `--coverage` stays relative to `root`.

- [x] **Step 1: Write the failing tests**

Move `fixture` in `tests/check_command.rs` to `std::env::temp_dir()` so a fixture is not this repo.

Add:

- `a_run_from_a_subdirectory_scopes_the_diff` — cwd is `nested/`, `--json`. A touched over-gate function fails, an untouched over-gate function is inherited, a changed file outside `nested/` is absent, and the note is absent.
- `a_subdirectory_path_scopes_the_diff` — the same tree, invoked as `slopmeter nested` from the repo root.
- `a_subdirectory_run_reads_the_root_config_and_ignores_a_nested_one` — root `exclude = ["nested/wide.py"]`, nested `slopmeter.toml` excludes `nested/ok.py`, `--strict` from `nested/` passes and still scores `ok`.
- `a_subdirectory_run_honors_the_root_gitignore` — root `.gitignore` lists `secret.py`, `nested/secret.py` is over the gate, `--strict` from `nested/` passes.

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test check_command -- a_run_from_a_subdirectory a_subdirectory_path a_subdirectory_run_reads a_subdirectory_run_honors`

Expected: FAIL. The run notes that it is not a git repository, or the nested config and the secret file change the verdict.

- [x] **Step 3: Score from the git root's config, limited to the requested tree**

`score` resolves `repo_root`. A missing root keeps the given directory. `load_config` reads the git root, so a nested `slopmeter.toml` is ignored. Include and exclude patterns are matched against the repo-relative path. `select` walks the given directory and, inside a repo, honors parent `.gitignore` files. Report paths stay relative to the given directory. `partition` receives that directory. A coverage path from the config is joined to the repo; a `--coverage` flag is joined to the given directory.

- [x] **Step 4: Run the tests to verify they pass**

Run the Task 4 command, then `cargo test`. Expected: PASS.

- [x] **Step 5: Format and lint**

Run: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`. Expected: clean.
