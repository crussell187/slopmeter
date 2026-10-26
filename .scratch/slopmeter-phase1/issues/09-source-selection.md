# Source selection and test-file policy

Type: grilling
Status: open
Blocked by: 08

## Question

What is in the scored set by default? Per language, what are the default include globs (`src/`? the whole repo minus ignores?) and the default excludes (`target/`, `.venv/`, `build/`, vendored code, generated files)? Are test files scored for function-scope rules, excluded entirely, or scored with a separate profile — and how are tests detected (`tests/`, `test_*.py`, `#[cfg(test)]`, `*_test.cpp`)? Does the tool honour `.gitignore`? How does a user widen or narrow this in `slopmeter.toml`?
