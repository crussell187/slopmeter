# Diff Scope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `scope::partition` puts findings for touched functions in `findings` and the rest in `inherited`. `--strict` scores the whole set. An empty diff gates nothing.

**Architecture:** The diff is an input. `partition` reads git, then joins changed lines to the touch spans already on the IR. It does not measure, evaluate rules, or render. A pure deletion is joined to the touch span of the same function at the base revision, because that line is gone from the working tree. The binary stays empty until a later session wires the pipeline.

**Tech Stack:** Rust stable, edition 2021. Git on `PATH`. No new dependencies.

## Global Constraints

- The default diff runs from the merge-base of `HEAD` and `main` through the working tree, including staged changes. `--changed=<ref>` picks another base. `--changed=HEAD` is only what is new since the last commit. On `main` itself, only uncommitted changes count.
- If `main` is missing, the base is `master`. If neither exists, or the merge-base cannot be computed, the run fails and asks for `--changed=<ref>` or `--strict`.
- A directory that is not a git repo is scored in full, and the run says so. `partition` treats `root` as the work tree and does not walk up to a parent repository.
- A function is touched when a changed line hits its touch span. A change elsewhere in the file does not touch it. A pure rename with no changed line inside the span does not touch it. An untracked file is touched in full.
- A deleted line touches the function whose touch span contained it at the base. Deleting a blank line or a comment outside every span touches nothing.
- `--strict` puts every finding in `findings` and does not need a base. A touched function contributes every one of its findings, gate or warn. Lists keep the incoming order.
- The missing-base error is `no base ref; pass --changed=<ref> or --strict`. A bad ref is `changed ref <ref> is not a revision; pass --changed=<ref> or --strict`. The outside-git note is `not a git repository; scoring the whole tree`.

---

### Task 1: Split findings on changed lines

**Files:**
- Create: `src/scope.rs`
- Modify: `src/lib.rs`
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `ir::File` (`path`, `touch`), `rules::Finding` (`path`, `name`, `param_types`, `qualifier`)
- Produces:

```rust
pub struct ScopeOptions {
    pub changed: Option<String>,
    pub strict: bool,
}

pub struct Scoped {
    pub scope: reporters::Scope,
    pub findings: Vec<Finding>,
    pub inherited: Vec<Finding>,
    pub notes: Vec<reporters::Note>,
}

pub fn partition(
    root: &Path,
    files: &[ir::File],
    findings: &[Finding],
    options: ScopeOptions,
) -> Result<Scoped, ScopeError>
```

- [x] **Step 1: Write the failing tests**

A fixture repo on `main` with one changed line puts that function's findings in `findings` and another over-threshold function in `inherited`. `--strict` puts both in `findings`. A warn on the touched function moves with it. A line outside every span, an empty diff, a staged edit, a pure rename, and a commit already on `main` gate nothing. A rename that also edits a line gates that function. A change in another file does not.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib scope::`
Expected: FAIL to compile because `scope` has no `partition`.

- [x] **Step 3: Join added and modified lines to touch spans**

`git diff --unified=0 --find-renames` against `merge-base(HEAD, main)`. A hunk with a positive new-file count marks those lines. A finding follows its function when any marked line lies in that function's touch span. `--strict` copies every finding and skips git.

- [x] **Step 4: Run the tests**

Run: `cargo test --lib scope::`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/scope.rs
git commit -m "Gate findings to functions the diff touches."
```

---

### Task 2: Base ref, untracked files, and no repository

**Files:**
- Modify: `src/scope.rs`
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `ScopeOptions::changed`
- Produces: `ScopeError::NoBase`, `ScopeError::BadRef`, and the outside-git note. Untracked paths come from `git status --porcelain=v1 -z -uall`.

- [x] **Step 1: Write the failing tests**

An untracked file is touched. A directory that is not a git repo scores every finding and records the note. No `main` or `master` returns the missing-base error. `master` is the base when `main` is absent. A commit on a branch is touched; `--changed=HEAD` with a clean work tree is not. An unknown ref returns the bad-ref error. `--strict` still scores a repo that has neither base.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib scope::tests::an_untracked_file_is_touched -- --exact`
Expected: FAIL because the untracked function is inherited.

- [x] **Step 3: Resolve the base and the untracked set**

Check `main`, then `master`, then merge-base. Honor `--changed` after it resolves. Set `GIT_CEILING_DIRECTORIES` to the parent of `root` and require the toplevel to be `root`. A missing toplevel scores the whole set with the note.

- [x] **Step 4: Run the tests**

Run: `cargo test --lib scope::`
Expected: PASS.

---

### Task 3: A deleted line

**Files:**
- Modify: `src/scope.rs`
- Test: `src/scope.rs`

**Interfaces:**
- Consumes: `adapters::python::parse` for the blob at the base
- Produces: the same `partition` result. Rust and C++ have no parser yet; a deletion in those files touches every function in the file.

- [x] **Step 1: Write the failing tests**

Deleting a line inside a function touches it, including its last line. Deleting the blank line between two functions touches neither. Deleting a comment above a function does not touch it.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib scope::tests::deleting_a_line_inside_the_function_touches_it -- --exact`
Expected: FAIL because the function is inherited.

- [x] **Step 3: Join deleted lines to the base touch span**

A hunk with a positive old-file count marks those base lines. `git show <base>:<old path>` is parsed with the file's language. The same name, parameter types, and qualifier carry the touch onto the working-tree function.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/scope.rs
git commit -m "Gate findings to functions the diff touches."
```
