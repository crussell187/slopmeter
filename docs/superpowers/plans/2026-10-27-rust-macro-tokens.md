# Rust macro tokens Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Score a Rust macro from its token tree, on the function that contains it, per ADR 0024.

**Architecture:** `walk_macro` today adds one decision for an `assert*` name and then stops. Keywords and punctuation inside a `token_tree` are anonymous nodes, so `named_nodes` never sees `if`, `&&`, or `.`. A walker over every child, anonymous included, records decisions, cognitive increments, and member chains. It does not increment `lloc` for those tokens. `visit` records a module-level `macro_definition` as a function and records an `fn` with a `{…}` body inside a scored macro as its own function. `classify` grows a `token_tree` arm so those bodies are scored at depth 0.

**Tech Stack:** Rust, tree-sitter-rust 0.24.2, adapter tests in `src/adapters/rust.rs`.

## Global Constraints

- The rule is [A Rust macro is scored from its tokens](../../adr/0024-rust-macro-tokens.md). `|` adds nothing. A module-level invocation is not a function. Logical lines inside a macro do not change.
- `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, and `debug_assert_ne` stay one decision and no cognitive increment, and their arguments still count.
- A listed allow-list name adds no hop. A trailing run of calls that ends in `build` adds none. Both stay in `analyzers::measure`.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass.

---

### Task 1: Score an invocation on the enclosing function

**Files:**
- Modify: `src/adapters/rust.rs` (`walk_macro`, new token walk, and a test in the adapter `tests` module)
- Test: `src/adapters/rust.rs`

**Interfaces:**
- Consumes: `Classifier::decision`, `Classifier::structural`, `Classifier::hybrid`, `Classifier::fundamental`, `macro_name`, `ASSERTS`
- Produces: `Classifier::walk_macro` walks the invocation's `token_tree`. Anonymous node kinds are the token text (`"if"`, `"&&"`, `"{"`).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_macro_invocation_scores_on_the_enclosing_function() {
    let source = "\
fn check(a: i32) -> bool {
    dbg!(if a > 0 { true } else { false })
}

fn bits(a: i32) -> i32 {
    dbg!(a | 1)
}

fn asserted(a: bool, b: bool) {
    assert!(a && b);
}

lazy_static! {
    static ref FLAG: bool = if ready() { true } else { false };
}
";
    let file = parse("a.rs", source.as_bytes());
    let check = function(&file, "check");
    assert_eq!(check.body.decisions.len(), 1);
    assert_eq!(cognitive_total(&check.body), 2);
    assert_eq!(function(&file, "bits").body.decisions.len(), 0);
    assert_eq!(function(&file, "asserted").body.decisions.len(), 2);
    assert!(file.functions.iter().all(|function| function.name != "lazy_static"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::rust::tests::a_macro_invocation_scores_on_the_enclosing_function -- --exact`
Expected: FAIL. `check` has no decisions. `asserted` has 1.

- [ ] **Step 3: Walk every child of the token tree**

Replace `walk_macro` so an `assert*` name still records one decision and no cognitive increment, then walk the `token_tree` child at the current nesting. Iterate `node.child(i)`, not `named_nodes`. Recurse into a child whose kind is `token_tree`. On kind `"if"`, `"while"`, `"for"`, or `"loop"`, record one decision and one structural increment, and walk the following `{` `token_tree` at `nesting + 1`. On kind `"else"`, record one hybrid increment and walk its `{` group at `nesting + 1`. On kind `"&&"` or `"||"`, record one decision, and one fundamental increment when the operator differs from the previous boolean operator. Do not increment `lloc` for any of these tokens. Kind `"|"` records nothing.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib adapters::rust::tests::a_macro_invocation_scores_on_the_enclosing_function -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/adapters/rust.rs
git commit -m "fix: score Rust macro tokens on the enclosing function"
```

---

### Task 2: Match the else-if, match, guard, and parenthesis rules

**Files:**
- Modify: `src/adapters/rust.rs` (the token walk from Task 1, and a test)

**Interfaces:**
- Consumes: the token walk from Task 1
- Produces: `else if` is one decision plus the `else` hybrid. `match` is structural only. An `if` before `=>` is one decision and no cognitive increment. A `(` group resets the boolean-operator run.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn else_if_match_and_parentheses_follow_the_walker() {
    let source = "\
fn branched(a: bool, b: bool) -> i32 {
    dbg!(if a { 1 } else if b { 2 } else { 3 })
}

fn guarded(a: i32) -> i32 {
    dbg!(match a { 1 if a > 0 => 1, _ => 0 })
}

fn grouped(a: bool, b: bool, c: bool) -> bool {
    dbg!((a && b) && c)
}
";
    let file = parse("a.rs", source.as_bytes());
    let branched = function(&file, "branched");
    assert_eq!(branched.body.decisions.len(), 2);
    assert_eq!(cognitive_total(&branched.body), 3);
    let guarded = function(&file, "guarded");
    assert_eq!(guarded.body.decisions.len(), 1);
    assert_eq!(cognitive_total(&guarded.body), 1);
    let grouped = function(&file, "grouped");
    assert_eq!(grouped.body.decisions.len(), 2);
    assert_eq!(cognitive_total(&grouped.body), 2);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::rust::tests::else_if_match_and_parentheses_follow_the_walker -- --exact`
Expected: FAIL. `branched` counts the second `if` as structural, so its cognitive total is 4.

- [ ] **Step 3: Special-case the three shapes**

When the token before `if` is `else`, record one decision and do not record a structural increment. The `else` hybrid from Task 1 still applies. Kind `"match"` records one structural increment and no decision, and its `{` group does not gain a nesting level from `match` itself. An `if` whose group is followed by `=>` before the next `{` records one decision and no cognitive increment. Entering a `token_tree` whose first anonymous child is `"("` clears the previous boolean operator.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib adapters::rust::tests::else_if_match_and_parentheses_follow_the_walker -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/adapters/rust.rs
git commit -m "fix: count else-if, match, and guards inside Rust macros"
```

---

### Task 3: Record member chains inside the token tree

**Files:**
- Modify: `src/adapters/rust.rs` (the token walk, and a test)

**Interfaces:**
- Consumes: `Classifier::chains`, `Chain`, `Member`
- Produces: one `Chain` per `.` run. `rooted` is true when the run starts at a `self` node. A hop whose name is followed by `"("` has `call: true`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_dot_run_inside_a_macro_is_a_member_chain() {
    let source = "\
impl Repo {
    fn check(&self) {
        dbg!(self.a.b.c());
        let _ = vec![self.a.b, other.c.d];
    }
}
";
    let file = parse("a.rs", source.as_bytes());
    let check = function(&file, "Repo::check");
    assert_eq!(check.body.chains.len(), 3);
    assert!(check.body.chains[0].rooted);
    assert_eq!(member_names(&check.body.chains[0]), vec!["a", "b", "c"]);
    assert!(check.body.chains[0].members[2].call);
    assert_eq!(member_names(&check.body.chains[1]), vec!["a", "b"]);
    assert_eq!(member_names(&check.body.chains[2]), vec!["c", "d"]);
}
```

`member_names` is the helper already in this test module.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::rust::tests::a_dot_run_inside_a_macro_is_a_member_chain -- --exact`
Expected: FAIL. `chains` is empty.

- [ ] **Step 3: Break the run on the agreed tokens**

A `self` node or an `identifier` followed by a `"."` node starts a run. Each following `"."` plus a name appends a hop. The hop's `call` is true when the next token is `"("`. A comma, a semicolon, a `{` `token_tree`, `"::"`, `".."`, or a keyword pushes the run and starts a new one. `".."` and `"::"` are not hops.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib adapters::rust::tests::a_dot_run_inside_a_macro_is_a_member_chain -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/adapters/rust.rs
git commit -m "fix: record member chains inside Rust macro tokens"
```

---

### Task 4: Score a macro_rules body, and carve an fn out of a macro

**Files:**
- Modify: `src/adapters/rust.rs` (`visit`, `classify`, the token walk, and a test)

**Interfaces:**
- Consumes: `push_function`, `qualify`, `in_function`, `classify`
- Produces: a module-level `macro_definition` is a function. A `macro_definition` inside a function scores its right-hand token trees on that function. An `fn` identifier followed by a `{` `token_tree` is a separate function. `classify` accepts that `token_tree` as a body and walks it at nesting 0.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_macro_rules_body_is_scored_where_it_is_written() {
    let source = "\
macro_rules! hide {
    ($e:expr) => { if $e > 0 { true } else { false } };
}

fn check(a: i32) -> bool {
    hide!(a)
}

fn outer() {
    macro_rules! local {
        () => { if ready() { 1 } else { 0 } };
    }
    wrap!(fn inner(a: i32) -> i32 {
        if a > 0 { 1 } else { 0 }
    });
}
";
    let file = parse("a.rs", source.as_bytes());
    let hide = function(&file, "hide");
    assert_eq!(hide.body.decisions.len(), 1);
    assert_eq!(cognitive_total(&hide.body), 2);
    assert_eq!(function(&file, "check").body.decisions.len(), 0);
    let outer = function(&file, "outer");
    assert_eq!(outer.body.decisions.len(), 1);
    assert_eq!(cognitive_total(&outer.body), 2);
    let inner = function(&file, "outer::inner");
    assert_eq!(inner.body.decisions.len(), 1);
    assert_eq!(cognitive_total(&inner.body), 2);
    assert!(!outer.body.decisions.iter().any(|decision| decision.line == inner.body.decisions[0].line));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib adapters::rust::tests::a_macro_rules_body_is_scored_where_it_is_written -- --exact`
Expected: FAIL. `hide` is not a function. `outer` has no decisions.

- [ ] **Step 3: Discover the definition and the nested fn**

In `visit`, a `macro_definition` outside a function calls `push_function` with the macro's name field and the definition node as the body. A `macro_definition` inside a function is left for the enclosing body's walk. `classify` on a `macro_definition` walks each `macro_rule`'s `right` field and skips the `left` field. While walking tokens, an `"fn"` keyword, an `identifier`, and a later `{` `token_tree` push a function named by that identifier and do not score that group on the current function. Extend `classify` so a body whose kind is `token_tree` runs the token walk at nesting 0.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib adapters::rust::tests::a_macro_rules_body_is_scored_where_it_is_written -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/adapters/rust.rs
git commit -m "fix: score macro_rules bodies and fns nested in Rust macros"
```
