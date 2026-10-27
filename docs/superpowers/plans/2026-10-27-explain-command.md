# Explain Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `slopmeter explain <RULE>` prints that rule's shipped page and exits 0. An unknown rule, or the wrong number of arguments, writes the reason to stderr and exits 2.

**Architecture:** `cli::run` recognizes only the `explain` subcommand and returns a `Rendered`. `main` prints that result and returns its exit code. Any other invocation still does nothing, because scoring a tree is a later session. The page text stays `explain::page`. The command does not read a repo, a config file, or coverage.

**Tech Stack:** Rust stable, edition 2021. No new dependencies. The binary is tested with `std::process::Command`.

## Global Constraints

- `explain <rule>` prints one page on stdout and exits 0. The page is the shipped text from `docs/explain/`, including the shipped stand-in threshold. It does not read the project file.
- An unknown rule writes `unknown rule <id>` to stderr, prints nothing on stdout, and exits 2. The id comparison is case-sensitive. `f08` is unknown.
- `explain` with no rule writes `explain needs a rule`. `explain` with more than one argument writes `explain takes one rule`. Both print nothing on stdout and exit 2.
- A run that is not `explain` is still not a check. This session does not score a tree.

---

### Task 1: Print one page from the binary

**Files:**
- Create: `src/cli.rs`
- Create: `tests/explain_command.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Test: `tests/explain_command.rs`

**Interfaces:**
- Consumes: `explain::page(rule) -> Option<&'static str>`, `reporters::no_check(reason) -> Rendered`, `reporters::Rendered`
- Produces:

```rust
pub fn run(args: &[String]) -> Option<Rendered>
```

`Some` when the first argument is `explain`. `None` for every other invocation, which `main` turns into a silent success.

- [x] **Step 1: Write the failing tests**

`tests/explain_command.rs` runs the binary from an empty directory:

```rust
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    let dir = std::env::temp_dir().join(format!("slopmeter-explain-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(args)
        .current_dir(&dir)
        .output()
        .expect("run slopmeter")
}

#[test]
fn explain_prints_the_shipped_page_and_exits_0() {
    for rule in [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11",
    ] {
        let output = run(&["explain", rule]);
        assert_eq!(output.status.code(), Some(0), "{rule}");
        assert!(output.stderr.is_empty(), "{rule}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, slopmeter::explain::page(rule).unwrap());
        assert!(stdout.contains("Shipped stand-in:"), "{rule}");
    }
}

#[test]
fn an_unknown_rule_exits_2() {
    for rule in ["F09", "f08"] {
        let output = run(&["explain", rule]);
        assert_eq!(output.status.code(), Some(2), "{rule}");
        assert!(output.stdout.is_empty(), "{rule}");
        assert_eq!(output.stderr, format!("unknown rule {rule}\n").into_bytes());
    }
}

#[test]
fn explain_with_the_wrong_number_of_arguments_exits_2() {
    let missing = run(&["explain"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert_eq!(missing.stderr, b"explain needs a rule\n");

    let extra = run(&["explain", "F08", "F09"]);
    assert_eq!(extra.status.code(), Some(2));
    assert!(extra.stdout.is_empty());
    assert_eq!(extra.stderr, b"explain takes one rule\n");
}
```

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --test explain_command`
Expected: FAIL because stdout is empty and the exit code is 0.

- [x] **Step 3: Dispatch `explain` and print the page**

```rust
pub fn run(args: &[String]) -> Option<Rendered> {
    match args.first().map(String::as_str) {
        Some("explain") => Some(explain(&args[1..])),
        _ => None,
    }
}
```

A single known id returns the page and exit 0. No id returns `no_check("explain needs a rule")`. More than one argument returns `no_check("explain takes one rule")`. An id `page` does not know returns `no_check("unknown rule <id>")`. `main` prints both streams and returns the code. Any other argv returns success and prints nothing.

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add src/cli.rs src/lib.rs src/main.rs tests/explain_command.rs
git commit -m "Print one explain page from the explain command."
```
