---
status: accepted
date: 2026-10-27
---

# Cognitive complexity follows the white paper

F03 is not the path count. It is SonarSource's cognitive complexity, white paper v1.7, and the same table applies in Python, Rust, and C++. A nested break costs 1 plus its depth. An `else`, a run of like boolean operators, and a labelled jump cost 1 and do not take that depth. sonar-python's zeroes for `match` and for lambda nesting are gaps in that analyzer. Three increments the paper names are not used: recursion, because Phase 1 has no call graph; the extra depth on a nested function, because that function is scored on its own; and `#if`, because both arms are scored as text.

## Considered options

- **Copy each language's Sonar analyzer, gaps included.** Rejected. Python would then ignore `match` and would not nest a lambda, so the same shape would score differently in each language.
- **Count recursion from a direct call of the same name.** Rejected. A shared name is not a cycle, and an indirect cycle is invisible.
- **Give a nested function the paper's extra depth.** Rejected. The inner body is its own function. The decorator exception exists only to undo a cost we do not charge.
- **Count `#if` as a structural break.** Rejected. The paper's increment assumes one arm is kept. Phase 1 keeps both, except a literal `#if 0`.

## Consequences

- Depth starts at 0. An `if` (including `if let` and `if constexpr`), a ternary, a `match` or `switch`, a loop (`for`, `while`, `do`, `loop`, range-`for`, `while let`), and a `catch` are worth 1 plus their depth, and they raise depth for what is inside them. One `catch` is one increment however many types it names. A `match` or `switch` is one increment. Its labels, a `default`, a `_`, and a guard add nothing.
- `elif`, `else if`, and `else` are worth 1. So is the `else` on a `for`, a `while`, or a `try`. Each raises depth for its body and does not add its own depth. `if` / inner `if` / `elif` is 4. `if` / `else` / inner `if` is 4.
- A run of `and`/`or` or `&&`/`||` is worth 1. A new run starts when the operator changes. Parentheses start a new expression. Negation does not break a run. Nesting does not multiply it. It counts in a condition, an assignment, a return, and a call argument. A bitwise `&` or `|` does not count.
- A labelled jump is worth 1 and is not multiplied by depth: `goto`, `break` or `continue` to a label, and a numbered `break` or `continue`. A plain `break`, `continue`, or `return` adds nothing. Python has no labelled form.
- A lambda or closure adds nothing and raises depth for its body, which stays on the enclosing function. A nested `def`, `fn`, or local-class method starts at depth 0, and its increments stay on it.
- These add nothing and raise no depth: `try`, `finally`, `with`, a comprehension's `for` and `if`, Rust `?`, `assert`, `raise`, `yield`, `await`, and recursion. A boolean sequence inside a comprehension still counts. `#if`, `#ifdef`, and `#elif` add nothing. Both arms are scored, and a literal `#if 0` still drops that arm.
- The function carries those increments as line, kind (`structural`, `hybrid`, `fundamental`), and depth. The measurement is their sum: 1 plus depth for structural, and 1 for the other two. The finding is that total, at the `def` or `fn` line. The threshold stays with calibration.
