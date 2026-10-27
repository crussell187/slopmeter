# Source Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `inputs::select` returns the default scored set for a tree: claimed files, minus gitignore, denylisted directories, protobuf names, and tests, with `--tests` putting tests back.

**Architecture:** One module, `src/inputs.rs`. It walks the root with the `ignore` crate, turns gitignore on only when that root contains `.git`, and then applies the name rules from [ADR 0005](../../docs/adr/0005-scored-set.md). Include and exclude globs stay in the config session. `#[cfg(test)]` and test attributes stay in the Rust adapter, because they are items inside a file, not files.

**Tech Stack:** Rust stable, edition 2021, `ignore` 0.4.33.

## Global Constraints

- One crate, `slopmeter`. This session adds the `ignore` dependency and no others.
- Paths in the result are repo-relative, use `/`, and are sorted.
- A directory named `target`, `.venv`, `venv`, `__pycache__`, `build`, `dist`, `vendor`, `third_party`, `third-party`, or `node_modules` is out, even when committed. A file named `build.rs` stays in.
- Protobuf names out: `*_pb2.py`, `*_pb2_grpc.py`, `*.pb.cc`, `*.pb.cpp`, `*.pb.h`, `*.pb.hpp`, and the same four with `.grpc.pb`.
- A test file is one under a `test` or `tests` directory, or `test_*.py`, `*_test.py`, `conftest.py`, or a claimed C++ file whose stem ends in `_test`, `_tests`, or `_unittest`. `testing/` is not a test directory. `WidgetTest.cpp` is out only under `tests/`.
- Outside a git repo, a `.gitignore` file does not apply. Inside one, a file git would ignore is out.
- Claimed extensions: `.py`, `.rs`, `.cpp`, `.cc`, `.cxx`, `.c++`, `.h`, `.hh`, `.hpp`, `.hxx`, `.inl`, `.ipp`, `.cppm`, `.ixx`.
- `examples/` and `benches/` stay in.

---

### Task 1: Select the scored files

**Files:**
- Modify: `Cargo.toml` (`ignore` = `0.4.33`)
- Modify: `src/lib.rs`
- Create: `src/inputs.rs`
- Test: `src/inputs.rs`

**Interfaces:**
- Consumes: `slopmeter::ir::Language`
- Produces: `slopmeter::inputs::{select, SelectOptions, Selected, SelectError}`

`select(root, options) -> Result<Vec<Selected>, SelectError>`. `SelectOptions` has `tests: bool`. `Selected` has `path: String` and `language: Language`.

- [x] **Step 1: Write the failing test**

`src/lib.rs`:

```rust
pub mod inputs;
pub mod ir;
```

`src/inputs.rs` contains only this module. `cargo test` fails because `select` is not defined.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Language;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("select-fixtures")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn paths(root: &Path, tests: bool) -> Vec<String> {
        select(root, SelectOptions { tests })
            .unwrap()
            .into_iter()
            .map(|s| s.path)
            .collect()
    }

    #[test]
    fn the_default_set_keeps_source_and_build_rs() {
        let root = fixture("default");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "tests/test_a.py", "x = 1\n");
        write(&root, "target/b.rs", "fn b() {}\n");
        write(&root, "foo.pb.cc", "void foo() {}\n");
        write(&root, "build.rs", "fn main() {}\n");

        let selected = select(&root, SelectOptions { tests: false }).unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|s| s.path.as_str())
                .collect::<Vec<_>>(),
            vec!["build.rs", "src/a.py"]
        );
        assert_eq!(selected[0].language, Language::Rust);
        assert_eq!(selected[1].language, Language::Python);
    }

    #[test]
    fn the_tests_switch_brings_test_files_back() {
        let root = fixture("tests-switch");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "tests/test_a.py", "x = 1\n");
        write(&root, "target/b.rs", "fn b() {}\n");
        write(&root, "foo.pb.cc", "void foo() {}\n");
        write(&root, "build.rs", "fn main() {}\n");

        assert_eq!(
            paths(&root, true),
            vec![
                "build.rs".to_string(),
                "src/a.py".to_string(),
                "tests/test_a.py".to_string(),
            ]
        );
    }
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib the_default_set_keeps_source_and_build_rs`
Expected: FAIL to compile. `select` is not defined.

- [x] **Step 3: Write the walker and the name rules**

Implement `select` as specified in the global constraints. Gitignore stays off until the next test demands it.

- [x] **Step 4: Run the acceptance tests**

Run: `cargo test --lib inputs::`
Expected: PASS.

- [x] **Step 5: Add the gitignore tests and watch them fail**

A tree with `.git` and a `.gitignore` of `secret.py` must drop `secret.py`. The same tree without `.git` must keep `secret.py`.

- [x] **Step 6: Honor gitignore only inside a git repo**

Turn on `git_ignore`, `git_global`, and `git_exclude` when `root/.git` exists. Do not read parent directories' ignore files.

- [x] **Step 7: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/inputs.rs
git commit -m "Select the scored files."
```
