# Cognitive Complexity: SonarSource rules mapped to Python, Rust and C++

Resolves `.scratch/slopmeter-phase1/issues/03-cognitive-complexity.md` (rule F03).
Researched 2026-10-26 against the white paper and the analyzers' source code; nothing below is taken from secondary write-ups.

## Answer

The authoritative definition is SonarSource's white paper *Cognitive Complexity: a new way of measuring understandability* (G. Ann Campbell), currently **version 1.7, 29 August 2023** (the last substantive rule change was v1.5, April 2021). The score is a sum of four kinds of unit increments: **structural** (+1 for `if`/ternary, `switch`, loops, `catch`, each of which also raises the nesting level and receives a **nesting** increment equal to its depth inside other such structures or inside nested functions/lambdas), **hybrid** (+1 for `else if`/`elif`/`else`, which raise nesting level but get no nesting increment), and **fundamental** (+1 per run of like binary logical operators, per `goto`/labelled `break`/`continue`, and per function in a recursion cycle; never nested). Functions, lambdas, `try`/`finally`, early `return`, unlabelled `break`/`continue`, `switch` cases and null-coalescing are free. SonarSource's own open implementations for **Python** (sonar-python) and **Rust** (sonar-rust, SSAL-licensed, tree-sitter based) follow this closely but neither implements recursion; sonar-python additionally ignores `match`, lambdas and comprehensions as increment sources. SonarSource's **C/C++ analyzer is closed source** and its rule page is no longer published, so the community **sonar-cxx** plugin was used for C++; it follows the spec for `if`/`switch`/loops/`catch`/ternary/`goto`/`else`/lambdas but ignores preprocessor conditionals, recursion and macro-expanded code. Clippy's `cognitive_complexity` lint is **not** this metric (flat count of `if`/`match`, minus returns, no nesting) and should not be used as a reference; Mozilla's rust-code-analysis is a looser reimplementation with several deviations.

## General rules (white paper v1.7)

Page numbers refer to the PDF.

- **Three basic rules** (p. 5): ignore structures that shorthand multiple statements into one; +1 for each break in linear flow; +1 extra when flow-breaking structures are nested. Four increment types: Nesting, Structural (subject to nesting increment, increases nesting count), Fundamental (not subject to nesting), Hybrid (no nesting increment, but increases nesting count). "The type of an increment makes no difference in the math" (p. 6).
- **Ignore shorthand** (p. 6): no increment for a method itself, nor for null-coalescing operators (`a?.b`).
- **Structural increments** (p. 6): loops (`for`, `while`, `do while`, ...) and conditionals (ternary, `if`, `#if`, `#ifdef`, ...). **Hybrid increments** for `else if`, `elif`, `else`: "No nesting increment is assessed for these structures because the mental cost has already been paid when reading the `if`."
- **Catches** (p. 7): each `catch` is a structural increment regardless of how many exception types it names; "`try` and `finally` blocks are ignored altogether."
- **Switches** (p. 7): "A `switch` and all its cases combined incurs a single structural increment", explicitly unlike Cyclomatic Complexity.
- **Sequences of binary logical operators** (pp. 7-8): one fundamental increment per sequence of *like* operators; a new sequence starts each time the operator changes. `a && b && c && d` = +1; `a || b && c || d` = +3. Worked example: `if (a && b && c || d || e && f)` = +1 (if) +1 +1 +1. Negation does not break a sequence by itself but a parenthesised sub-expression does: `a && !(b && c)` = +2 for the operators. Applies in assignments, call arguments and `return` as well as conditions. Only *binary boolean* operators count (change log v1.1).
- **Recursion** (p. 8): a fundamental increment "for each method in a recursion cycle, whether direct or indirect".
- **Jumps to labels** (p. 8): `goto`, `break LABEL`, `continue LABEL`, `break N`, `continue N` are fundamental increments; "no other jumps or early exits cause an increment" (early `return`, plain `break`/`continue` are free).
- **Nesting level and nesting increments** (pp. 8-9, Appendix B pp. 16): the nesting level is raised by `if`/`else if`/`else`/ternary, `switch`, loops, `catch`, and "nested methods and method-like structures such as lambdas". A nesting increment "commensurate with its nested depth" is added to `if`, ternary, `switch`, loops and `catch` (not to `else`/`else if`). The method itself and `try` do not raise nesting. Lambdas: "+0 (but nesting level is now 1)". A top-level `#if DEBUG` is +1 and raises nesting for everything inside it (p. 9).
- **Else-if chains**: `if`/`else if`/`else` = +1/+1/+1 at the depth of the `if`; an `if` nested in an `else` is +2 at nesting 1 (test fixtures below confirm this in every implementation).
- **Compensating usages** (Appendix A, pp. 12-15): COBOL `ELSE IF`, JavaScript namespace-functions, and **Python decorators**: a function containing *only* a nested function and a `return` statement does not raise the nesting level for that nested function (`decorator_generator` example gives total 1).
- **Appendix B specification** (p. 16) is the concise list to implement from: increments for `if, else if, else, ternary; switch; for, foreach; while, do while; catch; goto LABEL, break LABEL, continue LABEL, break NUMBER, continue NUMBER; sequences of binary logical operators; each method in a recursion cycle`. The paper says atypical spellings such as `elif` are covered even if not listed.

## Python: SonarSource sonar-python (`CognitiveComplexityVisitor.java`)

Default threshold 15 (`CognitiveComplexityFunctionCheck.DEFAULT_THRESHOLD`). Code outside functions is also scored (fixture). Secondary-location message format: `+N (incl N-1 for nesting)`.

| Construct | Increment | Notes (source / fixture) |
|---|---|---|
| `if` | structural + nesting | `visitIfStatement`, `isElif()` false |
| `elif` | hybrid (+1, no nesting) | `isElif()` branch; body raises nesting (`elif_nesting` = 4) |
| `else` (of `if`, `for`, `while`, `try`) | hybrid (+1) | `visitElseClause`; `for_with_else`, `while_with_else`, `try_else` fixtures |
| `for`, `while` | structural + nesting | `visitForStatement`, `visitWhileStatement` |
| `except` | structural + nesting | `visitExceptClause`; tuple of types still +1 |
| `try`, `finally` | none; neither raises nesting | `TRY_STMT`, `FINALLY_CLAUSE` in `notIncrementingNestingKinds` |
| `with` | none; does not raise nesting | `WITH_STMT` excluded explicitly |
| `x if c else y` | structural + nesting; raises nesting for its operands | `visitConditionalExpression`; nested ternary = +1 +2 |
| `and` / `or` sequences | fundamental, +1 per operator change | `flattenOperators` through AND/OR only; `1 and 2 or 3 and 4` = +3; `not` is not an operator here |
| `return`, `break`, `continue` | none | `jump_statements` fixture |
| nested `def` | +0; raises nesting by 1 | `NestingLevel(parent, FUNCDEF)`: `parent.level + 1` |
| decorator wrapper (`def` whose siblings are only `return NAME`) | +0; does **not** raise nesting | `isWrapperFunction` / `isSimpleReturn`; implements Appendix A |
| nested `class` | resets nesting to 0 | `NestingLevel` for CLASSDEF: `level = 0`; `function_with_nested_class` fixture |
| `lambda` | none; does **not** raise nesting | no `visitLambda` override; operators/ternaries inside still count at the enclosing level |
| list/set/dict comprehension, generator | none for `for`/`if` clauses | no handling of `COMP_FOR`/`COMP_IF`; a ternary or `and`/`or` inside still counts |
| `match` / `case` | **none** for the statement; `case` bodies raise nesting (inferred: `CASE_BLOCK` not in the exclusion list of `isStmtListIncrementsNestingLevel`) | no `MATCH_STMT` handling; not covered by any fixture |
| recursion | **not implemented** | fixture comment `# TODO recursion +1` |
| `assert`, `raise`, `yield`, `await` | none | not visited |

## Rust: SonarSource sonar-rust (`analyzer/src/visitors/cognitive_complexity.rs`)

Tree-sitter based; the rule test `test_default_threshold` constructs the check with 15. Clippy and rust-code-analysis are in the last two columns for comparison.

| Construct | sonar-rust increment | rust-code-analysis (Mozilla) | clippy `cognitive_complexity` |
|---|---|---|---|
| `if` / `if let` (not else-if) | structural + nesting | structural + nesting | flat +1 per `if` |
| `else if` | hybrid +1 (the `else` keyword); the inner `if` is skipped (`is_else_if`) | +1 for `Else` node; inner `if` skipped | +1 (it is an `if`) |
| `else` | hybrid +1 | +1 | none |
| `while`, `while let`, `for` | structural + nesting | structural + nesting | none |
| `loop` | structural + nesting | **none** (no `LoopExpression` arm) | none |
| `match` | structural + nesting (+0 per arm; guards free) | structural + nesting | +1 if >1 arm, +1 per guarded arm |
| `break 'label` / `continue 'label` | fundamental +1 (`label` child of break/continue) | +1 | none |
| plain `break`/`continue`/`return` | none | none | `return`s *subtract* (`returns/2`, or `returns` for `Result` fns) |
| `?` operator | none (no `try_expression` arm) | none | none |
| `&&` / `\|\|` sequences | fundamental; `a \|\| b && c \|\| d` = 3 (`test_binary_operators`) | +1 per operator change, but tracked as a running state reset at each nesting structure rather than per expression | none |
| closure | +0; raises nesting by 1 (`closure_expression`) | +0; raises nesting ("lambda" counter) | body skipped (`for_each_expr_without_closures`) |
| nested `fn` | +0; raises nesting by 1 when inside another fn (`current_enclosing_functions > 0`); no decorator-style exception | +0; raises nesting (`depth`) | separate function |
| `async` block, `unsafe` block | none | none | none |
| recursion | **not implemented** (`TODO SKUNK-29`) | not implemented (TODO at top of file) | none |
| macros | not special-cased (tree-sitter sees the invocation as a token tree) | `//TODO: Implement macros` | expansions skipped |
| `#[test]` fns | scored | scored | skipped |

Clippy's lint starts at 1, adds per `if`/`match`, subtracts returns, has no nesting term, default threshold 25, lives in the `restriction` group, and its own docs say "that's not what this lint does" about measuring cognitive complexity. Treat it as unrelated.

## C++: community sonar-cxx (`CxxCognitiveComplexityVisitor.java`)

Which implementation and why: SonarSource's C/C++ analyzer (SonarCFamily) is closed source; a SonarSource employee confirms on the community forum that it ships rule S3776 since SonarCFamily 4.6 and reports per-line increments only when the threshold is exceeded, but gives no construct-level detail. The former rule page (`rules.sonarsource.com/cpp/RSPEC-3776`) now 404s and the site states "The standalone rules website is no longer available"; the public `SonarSource/rspec` repository also 404s. So the table is from **sonar-cxx** (SonarOpenCommunity, LGPL; default `max` 15), cross-checked against clang-tidy's `readability-function-cognitive-complexity`, which states it implements the Sonar specification v1.2 (default threshold 25).

| Construct | sonar-cxx increment | Notes |
|---|---|---|
| `if` (incl. `if constexpr`) | structural + nesting | `selectionStatement` in `NESTING_INCREMENTS_TYPES` |
| `else if` | hybrid +1 (`ELSE` token); nested `if` skipped via `isElseIf` | fixture `+1: else statement` |
| `else` | hybrid +1 | `ELSE` in `INCREMENT_TYPES` only |
| `switch` | structural + nesting; cases free | same `selectionStatement` node; fixture `+1: switch statement` |
| `while`, `do while`, `for`, range-`for` | structural + nesting | `iterationStatement` covers all four |
| `catch` (incl. `catch(...)`) | structural + nesting | `handler`; `try` is nothing and raises no nesting |
| `cond ? a : b` | structural + nesting; raises nesting | `CxxPunctuator.QUEST` in both nesting lists; fixture `+3: conditional operator (incl 2 for nesting)` |
| `goto` | fundamental +1 | `GOTO` in `INCREMENT_TYPES` |
| `break`, `continue`, `return` | none | not subscribed |
| `&&` / `\|\|` sequences | fundamental, +1 per `logicalAndExpression` / `logicalOrExpression` node | Grammar nodes exist only when an operator is present (`skipIfOneChild`) and absorb a whole run of like operators, so `a && b && c` = +1. **Inferred deviation**: `a \|\| b && c \|\| d` parses as one `logicalOr` node containing one `logicalAnd` node = +2, not the spec's +3 (not covered by fixtures) |
| lambda `[]{}` | +0; raises nesting | `lambdaExpression` in `NESTING_LEVEL_TYPES` only |
| nested function / local class method | each `functionBody` opens its own scope; outer scopes' nesting is incremented for all open scopes (`for (var scope : complexityScopes)`) | |
| `#if` / `#ifdef` | **none** (spec says +1 structural) | preprocessor is not in the AST; clang-tidy documents the same omission |
| code from macro expansion | **ignored** (`isGeneratedCode()` early return) | fixture: `func_with_macro` raises no issue; clang-tidy counts macro code by default (`IgnoreMacros=false`) |
| recursion | **not implemented** | clang-tidy: "can't be fully implemented, because cross-translational-unit analysis would be needed" |
| `co_await`/`co_return`, `and`/`or` alternative tokens | alternative tokens map to the same grammar nodes; coroutine keywords are nothing | grammar `b.firstOf("&&", CxxKeyword.AND)` |

Mozilla rust-code-analysis for C++ counts `if`/`for`/`while`/`do`/`switch`/`catch` with nesting, `goto` and `else` +1, lambdas as nesting, but has **no arm for the ternary operator** and no preprocessor handling.

## Disagreements between implementations

1. **Recursion**: the white paper requires +1 per function in a cycle; none of sonar-python, sonar-rust, sonar-cxx, rust-code-analysis or clang-tidy implements it. The SonarSource rule text for Python and Rust still says recursive calls "will increment cognitive score".
2. **Lambdas raising nesting**: spec and sonar-rust, sonar-cxx, rust-code-analysis all do it; **sonar-python does not** (no `visitLambda`), so an `if`-expression inside a Python lambda is scored at the enclosing level.
3. **`match`/`switch`**: sonar-rust and sonar-cxx give +1 structural with nesting; **sonar-python gives `match` nothing** while still raising nesting inside `case` blocks (undocumented, inferred from source).
4. **Nested functions**: sonar-python has the decorator exception and resets nesting at a nested `class`; sonar-rust and sonar-cxx raise nesting for every nested function with no exception.
5. **`finally`**: spec ignores it; sonar-python ignores it; rust-code-analysis for Python gives `finally` +1 (`ElseClause | FinallyClause => increment_by_one`).
6. **`loop`**: sonar-rust counts it as a loop; rust-code-analysis does not.
7. **Ternary in C++**: sonar-cxx counts it with nesting (spec-conformant); rust-code-analysis does not count it at all.
8. **Mixed-operator sequences**: sonar-python and sonar-rust flatten the AND/OR tree and count operator changes exactly as the spec; sonar-cxx counts grammar nodes, which under-counts the `|| && ||` case by one; rust-code-analysis uses a stateful "previous operator" that is reset at nesting boundaries rather than per expression, so results differ across statements (its own Python test gives a 3-operator `or` chain inside two lambdas +3 total, i.e. nesting applied to a fundamental increment, which the spec forbids).
9. **Preprocessor conditionals and macros** (C++): spec counts `#if` as a structural conditional that raises nesting; no C++ implementation does. sonar-cxx drops macro-expanded code entirely; clang-tidy keeps it unless `IgnoreMacros`.
10. **Thresholds**: SonarSource analyzers default to 15 (Python, Rust, sonar-cxx); clippy and clang-tidy default to 25.
11. **Clippy** is a different metric altogether (flat `if`/`match` count minus returns, no nesting, closures skipped).

## Sources (opened)

- White paper v1.7 (PDF): https://www.sonarsource.com/docs/CognitiveComplexity.pdf
- Rules site retirement notice: https://www.sonarsource.com/products/sonarqube/rules/ (redirect target of https://rules.sonarsource.com/; `https://rules.sonarsource.com/cpp/RSPEC-3776/` and `https://github.com/SonarSource/rspec/tree/master/rules/S3776` both return 404)
- SonarSource community thread on C/C++ S3776: https://community.sonarsource.com/t/see-cognitive-complexity-scoring-in-c-source/18343
- sonar-python visitor: https://raw.githubusercontent.com/SonarSource/sonar-python/master/python-frontend/src/main/java/org/sonar/python/metrics/CognitiveComplexityVisitor.java
- sonar-python fixture: https://raw.githubusercontent.com/SonarSource/sonar-python/master/python-frontend/src/test/resources/metrics/cognitive-complexities.py
- sonar-python check (threshold): https://raw.githubusercontent.com/SonarSource/sonar-python/master/python-checks/src/main/java/org/sonar/python/checks/CognitiveComplexityFunctionCheck.java
- sonar-python rule text: https://raw.githubusercontent.com/SonarSource/sonar-python/master/python-checks/src/main/resources/org/sonar/l10n/py/rules/python/S3776.html
- sonar-python tree kinds: https://raw.githubusercontent.com/SonarSource/sonar-python/master/python-frontend/src/main/java/org/sonar/plugins/python/api/tree/Tree.java
- sonar-rust visitor: https://raw.githubusercontent.com/SonarSource/sonar-rust/master/analyzer/src/visitors/cognitive_complexity.rs
- sonar-rust check: https://raw.githubusercontent.com/SonarSource/sonar-rust/master/analyzer/src/rules/cognitive_complexity_check.rs
- sonar-rust rule text: https://raw.githubusercontent.com/SonarSource/sonar-rust/master/sonar-rust-plugin/src/main/resources/org/sonar/l10n/rust/rules/rust/S3776.html
- sonar-cxx visitor: https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-squid/src/main/java/org/sonar/cxx/visitors/CxxCognitiveComplexityVisitor.java
- sonar-cxx scope: https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-squid/src/main/java/org/sonar/cxx/visitors/CxxComplexityScope.java
- sonar-cxx grammar: https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-squid/src/main/java/org/sonar/cxx/parser/CxxGrammarImpl.java
- sonar-cxx check and tests: https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-checks/src/main/java/org/sonar/cxx/checks/metrics/FunctionCognitiveComplexityCheck.java, https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-checks/src/test/java/org/sonar/cxx/checks/metrics/FunctionCognitiveComplexityCheckTest.java, https://raw.githubusercontent.com/SonarOpenCommunity/sonar-cxx/master/cxx-checks/src/test/resources/checks/FunctionCognitiveComplexity.cc
- rust-code-analysis: https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/metrics/cognitive.rs, https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/checker.rs
- clippy lint source and docs: https://raw.githubusercontent.com/rust-lang/rust-clippy/master/clippy_lints/src/cognitive_complexity.rs, https://clippy.rust-lang.org/master/index.html
- clang-tidy check: https://clang.llvm.org/extra/clang-tidy/checks/readability/function-cognitive-complexity.html
