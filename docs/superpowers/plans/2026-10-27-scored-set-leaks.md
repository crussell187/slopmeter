# Scored-Set Leaks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A default run skips `fuzz/` and `testing/` directories and skips Rust `#[cfg(test)]` items and `#[test]` functions; `--tests` brings all of those back.

**Architecture:** Directory segments stay in `inputs::select`, beside `test` and `tests`, so `--tests` already turns them back on. Rust test items stay in `adapters::rust`: `parse` drops them, and `parse_with(..., include_tests)` keeps them. `check::score` passes the CLI flag through `adapters::parse`. Python and C++ parse signatures stay `parse(path, source)`.

**Tech Stack:** Rust stable, tree-sitter-rust 0.24.2, `cargo test`.

## Global Constraints

- `#[cfg(test)]` modules are out, helpers included (ADR 0005). A `#[test]` function is out the same way.
- Only the attribute `#[test]` with no arguments, and `#[cfg(test)]` whose argument is the single identifier `test`. `#[cfg(not(test))]` and `#[tokio::test]` stay scored.
- `fuzz` and `testing` are directory segments in `is_test_file`, not names in the always-denied list. `target/` stays out even with `--tests`. A file named `testing.rs` stays in.
- `adapters::rust::parse(path, source)` keeps the contract and excludes tests. The check passes `include_tests` from `--tests`.

---

### Task 1: Exclude fuzz and testing directories

**Files:**
- Modify: `src/inputs/select.rs:143-162`
- Test: `src/inputs/select.rs` (unit tests in the same file)

**Interfaces:**
- Consumes: `select(root, SelectOptions { tests })`
- Produces: the same `select` signature. `fuzz` and `testing` segments follow `test` and `tests`.

- [x] **Step 1: Write the failing test**

Add this test next to `the_tests_switch_keeps_test_names_and_still_drops_protobuf`.

```rust
#[test]
fn fuzz_and_testing_directories_stay_out_until_tests() {
    let root = fixture("fuzz-testing");
    write(&root, "src/lib.rs", "fn kept() {}\n");
    write(&root, "src/testing.rs", "fn tool() {}\n");
    write(&root, "fuzz/harness.rs", "fn fuzz() {}\n");
    write(&root, "testing/wide.py", "x = 1\n");
    write(&root, "src/testing/a.py", "x = 1\n");
    write(&root, "pkg/fuzz/a.py", "x = 1\n");

    assert_eq!(
        paths(&root, false),
        vec!["src/lib.rs".to_string(), "src/testing.rs".to_string()]
    );
    assert_eq!(
        paths(&root, true),
        vec![
            "fuzz/harness.rs".to_string(),
            "pkg/fuzz/a.py".to_string(),
            "src/lib.rs".to_string(),
            "src/testing.rs".to_string(),
            "src/testing/a.py".to_string(),
            "testing/wide.py".to_string(),
        ]
    );
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib inputs::select::tests::fuzz_and_testing_directories_stay_out_until_tests -- --exact`

Expected: FAIL because `fuzz/harness.rs` and `src/testing/a.py` are selected.

- [x] **Step 3: Write the minimal implementation**

In `is_test_file`, treat `fuzz` and `testing` like `test` and `tests`:

```rust
if segments.any(|segment| {
    matches!(segment, "test" | "tests" | "fuzz" | "testing")
}) {
    return true;
}
```

In `denylist_dirs_protobuf_and_unclaimed_extensions_drop`, drop `"src/testing/a.py"` from the expected keep list. That file is now a testing directory.

- [x] **Step 4: Run the select tests**

Run: `cargo test --lib inputs::select -- --test-threads=8`

Expected: PASS

### Task 2: Skip Rust test items unless --tests

**Files:**
- Modify: `src/adapters/rust.rs`
- Modify: `src/adapters/mod.rs`
- Modify: `src/check.rs`
- Test: `src/adapters/rust.rs`
- Test: `tests/check_command.rs`

**Interfaces:**
- Consumes: `leading_attributes`
- Produces: `rust::parse_with(path, source, include_tests) -> File`. `adapters::parse(language, path, source, include_tests)` passes the flag only for Rust.

- [x] **Step 1: Write the failing adapter test**

```rust
#[test]
fn a_cfg_test_module_and_a_test_function_are_not_scored() {
    let source = b"\
fn kept() {}

#[test]
fn eleven() {
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
}

#[cfg(test)]
fn hidden() {}

#[cfg(test)]
mod tests {
    fn helper() {}

    #[test]
    fn checked() {}
}

#[cfg(not(test))]
fn shipped() {}
";
    let file = parse("src/lib.rs", source);
    let names: Vec<_> = file.functions.iter().map(|function| function.name.as_str()).collect();
    assert_eq!(names, vec!["kept", "shipped"]);
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::rust::tests::a_cfg_test_module_and_a_test_function_are_not_scored -- --exact`

Expected: FAIL because `eleven`, `hidden`, `tests::helper`, and `tests::checked` are present.

- [x] **Step 3: Drop those items in visit**

`parse` calls `parse_with(path, source, false)`. `parse_with` threads `include_tests` into `visit`. At the start of `visit`, return without recording or descending when `!include_tests && is_test_item(node, source)`.

`is_test_item` is true for `function_item`, `mod_item`, `impl_item`, `trait_item`, `const_item`, and `static_item` when a leading attribute is `#[test]` with no arguments or `#[cfg(test)]` whose token tree has one named child, the identifier `test`.

- [x] **Step 4: Run the adapter test**

Expected: PASS, and `shipped` is still recorded.

- [x] **Step 5: Write the failing include-tests test and the command test**

The adapter test calls `parse_with("src/lib.rs", source, true)` and expects `kept`, `eleven`, `hidden`, `tests::helper`, `tests::checked`, and `shipped`.

The command test writes `src/lib.rs` with `fn kept() {}` and a `#[test] fn eleven` that has `Config::builtin().rust.f04_gate + 1` parameters and eleven `assert!(true);` lines. `--strict` exits 0 and does not print `eleven`. `--strict --tests` exits 1 and prints `eleven` and `F04`.

- [x] **Step 6: Wire the flag and re-run**

`adapters::parse` takes `include_tests: bool`. `check::score` passes `options.tests`. `parse_with(..., true)` records the test items.

Run: `cargo test --lib adapters::rust -- --test-threads=8` and `cargo test --test check_command -- --test-threads=8`

Expected: PASS

- [x] **Step 7: Format, clippy, and the full suite**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`

Expected: clean, and the git tests pass outside the sandbox.
