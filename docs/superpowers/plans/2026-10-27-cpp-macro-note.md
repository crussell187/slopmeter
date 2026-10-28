# Note unscored C++ function-like macros Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Note how many function-like `#define`s in a C++ file have unscored bodies, per ADR 0012.

**Architecture:** `File` gains `unscored_macros: u32`. The C++ adapter counts `preproc_function_def` nodes with the same arm rule as `visit_preproc`: a literal-`0` arm is skipped, every other arm counts. `check` turns a count above 0 into a `Note` beside the unparsed-ratio note. The note does not change the verdict. Python and Rust files carry 0.

**Tech Stack:** Rust, tree-sitter-cpp, adapter tests in `src/adapters/cpp.rs`, the check command test in `tests/check_command.rs`.

## Global Constraints

- The rule is [C++ is scored as text, and both arms of a conditional count](../../adr/0012-cpp-scored-as-text.md). A function-like `#define` is counted on its file. An object-like `#define` is not. A define in an arm whose condition is the token `0` is not. A define in any other arm is. The note does not fail the run. A use site scores the arguments written there.
- The singular note is `The parser left 1 function-like macro body unscored.` The plural note is `The parser left N function-like macro bodies unscored.`
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass.

---

### Task 1: Count function-like defines on the file

**Files:**
- Modify: `src/ir.rs` (`File`)
- Modify: `src/adapters/cpp.rs` (`parse`, new counter, and a test)
- Modify: `src/adapters/python.rs` and `src/adapters/rust.rs` (the `File` literals)
- Modify: `src/scope.rs`, `src/analyzers.rs`, `src/ir.rs` (test `File` literals)
- Test: `src/adapters/cpp.rs`

**Interfaces:**
- Consumes: `named_nodes`, `is_literal_zero`, `parse`
- Produces: `File.unscored_macros: u32`. `unscored_macros(node, source) -> u32` in `src/adapters/cpp.rs`. Python and Rust set the field to 0.

- [x] **Step 1: Write the failing test**

Add this test inside the adapter `tests` module in `src/adapters/cpp.rs`:

```rust
#[test]
fn a_function_like_define_is_counted_on_the_file() {
    let source = "\
#define MAX 3
#define CHECK(x) if (!(x)) return false;
#define NOOP(x)
#if 0
#define DEAD(x) if (x) return;
#else
#define ELSE_ARM(x) if (x) return;
#endif
#ifdef FLAG
#define A(x) if (x) return 1;
#else
#define B(x) if (x) return 2;
#endif
void local() {
#define INNER(x) if (x) return;
}
bool run(int x) { CHECK(x); return true; }
bool both(int a, int b) { return CHECK(a && b); }
";
    let file = parse("macros.hpp", source.as_bytes());
    assert_eq!(file.unscored_macros, 6, "{source}");
    assert_eq!(function(&file, "run").body.decisions.len(), 0);
    assert_eq!(function(&file, "both").body.decisions.len(), 1);
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::cpp::tests::a_function_like_define_is_counted_on_the_file -- --exact`

Expected: FAIL because `File` has no `unscored_macros` field.

- [x] **Step 3: Write the minimal implementation**

Add the field to `File` in `src/ir.rs`, after `unparsed_ratio`:

```rust
pub unscored_macros: u32,
```

Set `unscored_macros: 0` on every existing `File { ... }` literal in `src/ir.rs`, `src/adapters/python.rs`, `src/adapters/rust.rs`, `src/scope.rs`, and `src/analyzers.rs`, and on the early-return literal in `src/adapters/cpp.rs`.

In `parse`, set the field from the root:

```rust
unscored_macros: unscored_macros(root, source),
```

Add the counter next to `visit_preproc`. It uses the same dead-arm skip.

```rust
fn unscored_macros(node: Node, source: &[u8]) -> u32 {
    match node.kind() {
        "preproc_function_def" => 1,
        "preproc_def" | "preproc_include" | "preproc_call" | "comment" => 0,
        "preproc_if" | "preproc_elif" => unscored_in_preproc(node, source, true),
        "preproc_ifdef" | "preproc_elifdef" | "preproc_else" => {
            unscored_in_preproc(node, source, false)
        }
        _ => named_nodes(node)
            .into_iter()
            .map(|child| unscored_macros(child, source))
            .sum(),
    }
}

fn unscored_in_preproc(node: Node, source: &[u8], check_zero: bool) -> u32 {
    let condition = node.child_by_field_name("condition");
    let alternative = node.child_by_field_name("alternative");
    let dead = check_zero && condition.is_some_and(|condition| is_literal_zero(condition, source));
    let mut count = 0;
    if !dead {
        for child in named_nodes(node) {
            if Some(child) == condition || Some(child) == alternative {
                continue;
            }
            count += unscored_macros(child, source);
        }
    }
    if let Some(alternative) = alternative {
        count += unscored_macros(alternative, source);
    }
    count
}
```

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib adapters::cpp::tests::a_function_like_define_is_counted_on_the_file -- --exact`

Expected: PASS. `unscored_macros` is 6: `CHECK`, `NOOP`, `ELSE_ARM`, `A`, `B`, and `INNER`. `MAX` is object-like. `DEAD` sits in the literal-`0` arm. `run` has no decision. `both` has the `&&` written in the argument.

- [x] **Step 5: Commit**

```bash
git add src/ir.rs src/adapters/cpp.rs src/adapters/python.rs src/adapters/rust.rs src/scope.rs src/analyzers.rs
git commit -m "$(cat <<'EOF'
Count unscored C++ function-like macros on the file.

EOF
)"
```

---

### Task 2: Note the count without failing the run

**Files:**
- Modify: `src/check.rs` (`unparsed_notes`'s caller)
- Test: `tests/check_command.rs`

**Interfaces:**
- Consumes: `File.unscored_macros`, `Note { path, message }`
- Produces: a note after the unparsed-ratio notes when `unscored_macros > 0`. A count of 0 adds nothing. The verdict is unchanged.

- [x] **Step 1: Write the failing test**

Add this test in `tests/check_command.rs`:

```rust
#[test]
fn a_function_like_macro_is_noted_and_an_object_like_macro_is_not() {
    let root = fixture("macro-note");
    write(
        &root,
        "pkg/check.hpp",
        "\
#define CHECK(x) if (!(x)) return false;
bool run(int x) { CHECK(x); return true; }
",
    );
    write(
        &root,
        "pkg/two.hpp",
        "\
#define CHECK(x) if (!(x)) return false;
#define ALSO(x) if (x) return true;
int quiet() { return 1; }
",
    );
    write(
        &root,
        "pkg/const.hpp",
        "\
#define MAX 3
int ok() { return MAX; }
",
    );
    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["verdict"], "pass");
    let notes = doc["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert_eq!(notes[0]["path"], "pkg/check.hpp");
    assert_eq!(
        notes[0]["message"],
        "The parser left 1 function-like macro body unscored."
    );
    assert_eq!(notes[1]["path"], "pkg/two.hpp");
    assert_eq!(
        notes[1]["message"],
        "The parser left 2 function-like macro bodies unscored."
    );
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --test check_command a_function_like_macro_is_noted_and_an_object_like_macro_is_not -- --exact`

Expected: FAIL because `notes` is empty.

- [x] **Step 3: Write the minimal implementation**

In `src/check.rs`, extend the note list immediately after `unparsed_notes`:

```rust
let mut notes = unparsed_notes(&files);
notes.extend(macro_notes(&files));
```

Add the two functions beside `unparsed_notes`:

```rust
fn macro_notes(files: &[File]) -> Vec<Note> {
    files
        .iter()
        .filter(|file| file.unscored_macros > 0)
        .map(|file| Note {
            path: Some(file.path.clone()),
            message: macro_note(file.unscored_macros),
        })
        .collect()
}

fn macro_note(count: u32) -> String {
    if count == 1 {
        "The parser left 1 function-like macro body unscored.".to_string()
    } else {
        format!("The parser left {count} function-like macro bodies unscored.")
    }
}
```

`Note` is already rendered by the text table and the JSON report. This task does not add a reporter.

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo test --test check_command a_function_like_macro_is_noted_and_an_object_like_macro_is_not -- --exact`

Expected: PASS. The verdict is `pass`. `pkg/const.hpp` has no note. `pkg/check.hpp` carries the singular sentence.

- [x] **Step 5: Commit**

```bash
git add src/check.rs tests/check_command.rs
git commit -m "$(cat <<'EOF'
Note a C++ file whose function-like macros were not scored.

EOF
)"
```
