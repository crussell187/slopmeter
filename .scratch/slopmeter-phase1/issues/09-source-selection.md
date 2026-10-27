# Source selection and test-file policy

Type: grilling
Status: resolved
Blocked by: 08

## Question

What is in the scored set by default? Per language, what are the default include globs (`src/`? the whole repo minus ignores?) and the default excludes (`target/`, `.venv/`, `build/`, vendored code, generated files)? Are test files scored for function-scope rules, excluded entirely, or scored with a separate profile — and how are tests detected (`tests/`, `test_*.py`, `#[cfg(test)]`, `*_test.cpp`)? Does the tool honour `.gitignore`? How does a user widen or narrow this in `slopmeter.toml`?

## Answer

Resolved 2026-10-26 with Christopher. Why tests are out and the default is the whole repo: [ADR 0005](../../../docs/adr/0005-scored-set.md). The glossary term is **Scored set** in `CONTEXT.md`.

The default scored set is every claimed file in the repo, not only `src/`. With no config file, that default stands. Outside a git repo, only the built-in directory names below apply.

Out of the set:

- A file git would ignore.
- A directory named `target`, `.venv`, `venv`, `__pycache__`, `build`, `dist`, `vendor`, `third_party`, `third-party`, or `node_modules`, even when committed. A file named `build.rs` stays in.
- Protobuf output named `*_pb2.py`, `*_pb2_grpc.py`, `*.pb.cc`, `*.pb.cpp`, `*.pb.h`, `*.pb.hpp`, or the `.grpc.pb` forms. No other generated file is detected by name or by an `@generated` comment.
- Tests. A test is a file under a `test` or `tests` directory, `test_*.py`, `*_test.py`, `conftest.py`, or a C++ or header file ending in `_test`, `_tests`, or `_unittest`. A Rust `#[cfg(test)]` item takes everything nested in it. A function whose attribute ends in `test` is out, including `#[tokio::test]`. `#[rstest]` stays in. `WidgetTest.cpp` is out only under `tests/`. A directory named `testing` is not a test directory.

`examples/` and `benches/` stay in.

Include globs, when set, keep only matching files. Exclude globs remove more. Tests stay out until `--tests` turns them on. Including `target/` or `foo.pb.cc` does not bring them back. The keys and the flag are in [Shape of slopmeter.toml](22-config-shape.md).
