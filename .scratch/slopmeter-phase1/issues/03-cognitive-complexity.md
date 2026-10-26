# SonarSource cognitive complexity rules mapped to Python, Rust, C++

Type: research
Status: claimed
Context: research subagent fired 2026-10-26; findings land on branch `research/cognitive-complexity` at `docs/research/cognitive-complexity.md`
Blocked by: 

## Question

What are the exact rules of SonarSource's Cognitive Complexity metric (the 2017/2021 white paper): which constructs add a structural increment, which add a nesting increment, how nesting level is counted, how sequences of binary logical operators are scored, how recursion, lambdas/closures, and `else if`/`elif` chains are treated? Then, for Python, Rust and C++: how does SonarSource's own implementation for each language (sonar-python, the community Rust plugin if any, sonar-cxx or SonarSource's C++ analyzer) map those rules onto language constructs — comprehensions, `match` arms, `?`, `try/except`, ternaries, goto, lambdas, nested functions? Produce a table of construct → increment type per language, citing the white paper and the per-language rule docs or source.
