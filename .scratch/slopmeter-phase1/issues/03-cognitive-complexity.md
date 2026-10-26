# SonarSource cognitive complexity rules mapped to Python, Rust, C++

Type: research
Status: resolved
Context: research subagent fired 2026-10-26; findings land on branch `research/cognitive-complexity` at `docs/research/cognitive-complexity.md`
Blocked by: 

## Question

What are the exact rules of SonarSource's Cognitive Complexity metric (the 2017/2021 white paper): which constructs add a structural increment, which add a nesting increment, how nesting level is counted, how sequences of binary logical operators are scored, how recursion, lambdas/closures, and `else if`/`elif` chains are treated? Then, for Python, Rust and C++: how does SonarSource's own implementation for each language (sonar-python, the community Rust plugin if any, sonar-cxx or SonarSource's C++ analyzer) map those rules onto language constructs — comprehensions, `match` arms, `?`, `try/except`, ternaries, goto, lambdas, nested functions? Produce a table of construct → increment type per language, citing the white paper and the per-language rule docs or source.

## Answer

Resolved 2026-10-26 by a research subagent. Full findings: `docs/research/cognitive-complexity.md` on branch `research/cognitive-complexity` (read with `git show research/cognitive-complexity:docs/research/cognitive-complexity.md`).

- **Spec revision:** white paper v1.7 (29 Aug 2023); last rule change v1.5 (2021) made all labelled/multi-level `break`/`continue` fundamental increments. Appendix B (p. 16) is the list to implement from.
- **Four increment kinds:** structural (`if`/ternary, `switch`, loops, `catch`: +1 plus current nesting depth, and they raise nesting); hybrid (`else if`/`elif`/`else`: +1, raise nesting, no nesting increment); fundamental (+1 per run of like `&&`/`||`, `goto`/labelled jumps, each function in a recursion cycle; never nested). Functions, lambdas, `try`/`finally`, early `return` and `switch` cases are free; lambdas and nested functions raise nesting by one.
- **Python (sonar-python):** `if`/`for`/`while`/`except`/ternary structural+nesting; `elif`/`else` (incl. loop/try `else`) hybrid; `try`/`finally`/`with` neither count nor raise nesting; nested `def` raises nesting unless it is a decorator wrapper; nested `class` resets nesting to 0; lambdas do not raise nesting; comprehension clauses count nothing; `match` counts nothing (case bodies raise nesting). Recursion is a TODO. Threshold 15.
- **Rust (SonarSource/sonar-rust, tree-sitter based, official):** `if`/`while`/`loop`/`for`/`match` structural+nesting; `else`/`else if` hybrid; labelled `break`/`continue` +1; `&&`/`||` sequences per spec; closures and nested `fn` raise nesting; `?` and match guards free; recursion unimplemented. Threshold 15.
- **Clippy `cognitive_complexity` is a different metric:** flat 1 + per-`if` + per-multi-arm-`match` minus returns, no nesting, closures skipped, default 25, and its docs disclaim measuring cognitive complexity. Do not use it as a cross-check.
- **rust-code-analysis deviates:** no `loop`, no C++ ternary, Python `finally` +1, stateful boolean-sequence counter with nesting applied, recursion TODO.
- **C++:** SonarSource's CFamily analyzer is closed source and the public rules site/rspec repo are gone, so the community sonar-cxx plugin was used. It is spec-conformant for `if`/`switch`/loops/`catch`/ternary/`goto`/`else`/lambda nesting but ignores `#if`/`#ifdef`, drops macro-expanded code, has no recursion, and under-counts `a || b && c || d` as +2 (inferred). clang-tidy's check (spec v1.2, threshold 25) documents the same `#if` and recursion omissions.
- **Disagreements to decide in ticket 07/19:** recursion is unimplemented everywhere; lambda nesting (sonar-python no, others yes); `match`/`switch` (sonar-python nothing, Rust/C++ +1 with nesting); thresholds 15 (Sonar) vs 25 (clippy, clang-tidy). The spec's F03 defaults (gate > 15, warn > 10) match Sonar's.
