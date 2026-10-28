# Load-old all languages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A deleted line in a Rust or C++ file gates only the function whose old span contains that line.

**Architecture:** `load_old` already asks git for the pre-change bytes. It stops returning no functions for Rust and C++ and parses those bytes with `adapters::parse`, the same dispatcher the check uses. `deleted_hit` already matches old functions by name, parameter types, and qualifier, so no other scoring path changes.

**Tech Stack:** Rust, `adapters::parse`, git fixtures in `src/scope.rs` tests.

## Global Constraints

- A function is touched when a deleted line falls inside its span on the old side.
- Identity is name plus parameter types plus qualifier.
- Outside a parsed old file, a deletion still touches the whole file.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass.

---

### Task 1: Parse the old side in every language

**Files:**
- Modify: `src/scope.rs` (`load_old`, and the deletion test near the bottom of `tests`)

**Interfaces:**
- Consumes: `crate::adapters::parse(language, path, source, include_tests) -> Result<File, String>`
- Produces: `load_old(...) -> Option<Vec<Function>>` with one entry per function the adapter records, for Python, Rust, and C++. `include_tests` is `false`, matching `rust::parse`.

- [x] **Step 1: Write the failing tests**

Replace `a_deletion_in_an_unparsed_language_touches_the_file` with a Rust case and a C++ case. Each commits two functions, deletes one line inside the first, parses the working tree with that language's adapter, and expects only `touched` in `findings`.

```rust
#[test]
fn deleting_a_line_in_one_rust_function_touches_only_that_function() {
    let (root, file) = delete_lang(
        "rust-delete",
        "src/a.rs",
        "\
fn touched() {
    let n = 1;
    let _ = n;
}

fn other() {
    let _ = 0;
}
",
        "\
fn touched() {
    let _ = n;
}

fn other() {
    let _ = 0;
}
",
        Language::Rust,
    );
    assert_only_touched(&root, file);
}

#[test]
fn deleting_a_line_in_one_cpp_function_touches_only_that_function() {
    let (root, file) = delete_lang(
        "cpp-delete",
        "src/a.cpp",
        "\
int touched() {
    int n = 1;
    return n;
}

int other() {
    return 0;
}
",
        "\
int touched() {
    return n;
}

int other() {
    return 0;
}
",
        Language::Cpp,
    );
    assert_only_touched(&root, file);
}
```

`delete_lang` writes `before`, commits, writes `after`, and returns `adapters::parse` of the working-tree bytes. `assert_only_touched` partitions gates named `touched` and `other` and asserts those two lists.

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib deleting_a_line_in_one -- --test-threads=8`

Expected: both fail because `other` is in `findings`. `load_old` returns `None` for Rust and C++, so `OldSpans::Whole` treats any deletion as a hit on every function.

- [x] **Step 3: Parse every language**

```rust
fn load_old(root: &Path, base: &str, language: Language, old_path: &str) -> Option<Vec<Function>> {
    let prefix = work_tree_prefix(root).ok()?;
    let spec = format!("{base}:{prefix}{old_path}");
    let out = git(root, &["--no-pager", "show", &spec]).ok()?;
    if !out.status.success() {
        return None;
    }
    crate::adapters::parse(language, old_path, &out.stdout, false)
        .ok()
        .map(|file| file.functions)
}
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib scope::tests -- --test-threads=8`

Expected: the new tests pass, and the existing Python deletion tests still pass.

- [x] **Step 5: Commit**

```bash
git add src/scope.rs
git commit -m "Parse the old side of a deletion in Rust and C++."
```
