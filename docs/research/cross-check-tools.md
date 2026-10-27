# Cross-check tools: how radon, rust-code-analysis and lizard count cyclomatic complexity

Ticket: `.scratch/slopmeter-phase1/issues/04-cross-check-tools.md`. Researched 2026-10-26 against the tools' source and docs (versions: radon 6.0.1 on PyPI, lizard 1.24.3 on PyPI, rust-code-analysis `master`; the crates.io `rust-code-analysis-cli` 0.0.25 dates from 2023-01 and lags the repo).

## Answer

All three tools start every function at 1 and add 1 per decision, but they disagree at the edges, so no single one is a clean ruler for slopmeter. **radon** walks the Python AST: `if`/ternary, each loop (+1 more for a loop `else`), each `except` (+1 for `try…else`), `and`/`or` (n−1 per chain), each comprehension clause (+1 per `if` filter), non-wildcard `match` cases and `assert` count; `with` does *not* count in code even though its docs say +1, and `lambda`/closures are excluded. **rust-code-analysis** (Rust, C/C++, Python, Java, JS/TS, Kotlin) matches tree-sitter *keyword tokens*: Python counts `if/elif/for/while/except/with/assert/and/or` and loop-`else` but ignores `match`; Rust counts `if/for/while/loop`, every `match` arm (including `_`), `?` and `&&`/`||`; C++ counts `if/for/while/case/catch/?:/&&/||` (not `default`, not `do`'s own node, not `#if`). **lizard** is token-based: default `if for while catch && || case ?`; Python adds `elif except finally and or` and soft-keyword `case`; Rust adds `where` and `?`, counts match arms after the first via `=>`; C/C++ additionally counts `#if/#ifdef/#elif`. **crap4java** (Java) matches the ticket's summary: +1 per `if`, loop (incl. do-while), `catch`, `?:`, each `case` label (javac models `default` as a `CaseTree`, so it counts), `&&`, `||`. Beyond CC: radon gives file-level LLOC (statement-count with a `:` heuristic); rust-code-analysis gives per-function NARGS, NEXITS, cognitive complexity, LLOC/SLOC/PLOC/CLOC and function spans; lizard gives per-function NLOC, token count, parameter count, physical length, and nesting depth via `-END`/`-ENS`.

## radon (Python)

**Install / run**

```
pip install radon
radon cc -s -j -O cc.json src/        # per-block CC as JSON; -s adds the score to the rank
radon raw -j -O raw.json src/         # LOC/LLOC/SLOC/comments/multi/blank per file
```

`--no-assert` drops `assert` from CC; `-e`/`-i` exclude globs and ignore directories; `-n/-x` filter by rank; `-a`/`--total-average` print averages. The JSON is `{path: [block…]}`; each block (`radon/cli/tools.py::cc_to_dict`) carries `type` (`function`/`method`/`class`), `rank`, `name`, `lineno`, `col_offset`, `endline`, `complexity`, `classname`, plus nested `closures` and (for classes) `methods`. Rank A–F = 1–5, 6–10, 11–20, 21–30, 31–40, 41+ (`radon/complexity.py::cc_rank`).

**What increments CC** — `radon/visitors.py::ComplexityVisitor.generic_visit` / `visit_Assert`. Each function starts at `body_complexity = 1`.

| Construct | Increment |
|---|---|
| `If`, `IfExp` (ternary) | +1 each; `elif` is a nested `If`, so +1; `else` +0 |
| `For`, `While`, `AsyncFor` | `+ bool(node.orelse) + 1` (loop `else` adds 1) |
| `Try` | `+ len(node.handlers) + bool(node.orelse)`; `finally` +0 |
| `BoolOp` | `+ len(node.values) - 1` (`a and b and c` → +2) |
| `comprehension` (each `for` clause) | `+ len(node.ifs) + 1` |
| `Match` | `+ max(0, len(cases) - contains_wildcard)`: `case _` is free |
| `assert` | `+ not self.no_assert` |
| `with` | **+0 in code** (no `With` branch), although both the intro docs table and the in-code comment say "Ifs, with and assert statements count all as 1" |
| `lambda`, nested `def` | +0; "Lambda functions are not counted anymore, see #68"; closures are listed separately, not folded into the parent |

Class blocks: `1 + Σ(body complexity + method complexity + number of methods)`, and the reported class `complexity` is `int(real/methods) + (methods > 1)` — an average, not a sum. Do not compare slopmeter's class numbers to radon's.

**Other metrics** — `radon raw` is file-level only: LOC, LLOC, SLOC, comments, multi, single_comments, blank, with `SLOC + Multi + Comments + Blank = LOC`. LLOC (`radon/raw.py::_logical`) tokenizes each logical line, splits on `;`, and for each piece returns 2 if a `:` operator is followed by more tokens, 1 if the `:` is last or there is no colon, 0 if empty. A multi-line string or bracketed statement is one unit. Quirk: the colon test is not block-aware, so `d = {'a': 1}` and `x: int = 0` count 2. No parameter count, nesting depth or per-function length (use `endline - lineno + 1` from `cc -j`).

## rust-code-analysis (Rust, C/C++, Python and more)

**Install / run** — not archived (last push 2026-04); README documents building from source only:

```
git clone https://github.com/mozilla/rust-code-analysis && cd rust-code-analysis
cargo build -p rust-code-analysis-cli --release
rust-code-analysis-cli -m -p src/ -O json -o out/     # one JSON per input file
rust-code-analysis-cli -m -p src/ --pr -O json           # pretty JSON to stdout
```

Other flags (`rust-code-analysis-cli/src/main.rs`): `-l` language override, `-I`/`-X` include/exclude globs, `-j` jobs, `-F` function spans only, `--preproc` for C/C++ macro declarations. Output nests `FuncSpace`s (`src/spaces.rs`: `name`, `start_line`, `end_line`, `kind` unit/function/class/trait/impl, `spaces`, `metrics`). Languages (`src/langs.rs`): Mozjs/JavaScript, Java, Kotlin, Rust (`rs`), C/C++ (`c cc cpp cxx h hh hpp hxx inc m mm`), Python (`py`), TypeScript, TSX, plus the `Ccomment`/`Preproc` C variants — so it does cover all three slopmeter languages.

**What increments CC** — `src/metrics/cyclomatic.rs`. `Stats::default()` sets `cyclomatic: 1.`; every matching node adds `1.`. The serialized value per space is `{sum, average, min, max}`; the sum/average include the *unit* (file) space, which itself contributes 1 (tests annotate "+1 unit space"). Arms match tree-sitter **token kinds**, so e.g. `If` is the `if` keyword wherever it appears.

- Python: `If | Elif | For | While | Except | With | Assert | And | Or` +1; `Else` +1 only inside a `for`/`while`. Because keywords are matched, a ternary `if`, comprehension `for` and comprehension `if` also count (grammar inference; not covered by its tests). `match`/`case`, `lambda`, `finally`, `try…else`: +0. Each `and`/`or` token is +1 (equivalent to radon's n−1).
- Rust: `If | For | While | Loop | MatchArm | MatchArm2 | TryExpression | AMPAMP | PIPEPIPE` +1. Every match arm counts, including the first and `_` (test `rust_1_level_nesting`: `if` + 2 arms = 4); `loop {}` +1; `?` +1; `if let`/`while let` count via their keyword; `else`/`else if` +0 extra; closures +0.
- C/C++: `If | For | While | Case | Catch | ConditionalExpression | AMPAMP | PIPEPIPE` +1. `default` +0 (test `c_switch`: three `case` + `default` = 4); `switch`, `try`, `goto` +0; `do…while` has no arm of its own and is reached only through its `while` token; preprocessor conditionals +0 (handled by the separate `Preproc` grammar).

**Cognitive complexity** (`src/metrics/cognitive.rs`, Sonar-style, `{sum, average, min, max}`): nesting-increment constructs are Python `if/for/while/ternary/except`, Rust `if` (not `else if`)/`for`/`while`/`match`, C++ `if/for/while/do/switch/catch`; flat +1 for `elif/else/finally` (Py), `else`, labeled `break`/`continue` (Rust), `else`/`goto` (C++); `&&`/`||` (`and`/`or`, `not`) +1 per change of operator in a sequence; lambdas/closures raise nesting, a Rust `fn` item resets it. Rust `?` and macros are not scored (`//TODO: Implement macros`).

**Other metrics** — NARGS per function and per closure (`src/metrics/nargs.rs`: children of the `parameters` field that are not `is_non_arg`; C++ goes through the `declarator`), NEXITS, NOM, Halstead, MI, ABC, WMC, and the LOC suite: SLOC (all lines), PLOC (lines with code), LLOC (statement nodes — Rust: `Statement | EmptyStatement | ExpressionStatement | LetDeclaration | AssignmentExpression | CompoundAssignmentExpr`; Python and C++: their statement node kinds, listed in `src/metrics/loc.rs`), CLOC, BLANK, all per space. Function length is `end_line - start_line + 1` from the span. There is no nesting-depth metric.

## lizard (C/C++, Python, Rust and ~25 others)

**Install / run**

```
pip install lizard
lizard --csv -l cpp -l python -l rust src/ > lizard.csv   # per-function rows
lizard --xml src/ > lizard.xml                             # cppncss-style XML
```

CSV columns (`lizard_ext/csvoutput.py`): `NLOC, CCN, token, PARAM, length, location, file, function, long_name, start, end`. Useful flags (`docs/cli.rst`, `lizard_core/options.py`): `-C 15` CCN threshold (default 15), `-L 1000` length threshold, `-a 100` parameter threshold, `-T nloc=25` arbitrary field threshold, `-x "./build/*"` exclude, `-t N` threads, `-m/--modified` (whole `switch`/`match` = 1), `-E` extensions: `-END` nesting depth (threshold 7), `-ENS` nested control structures (threshold 3), `-Ecpre` ignore `#else` branches, `-EIgnoreAssert`, `-Ecognitive`, `-Ehalstead`, `-Eoutside` (global code as a function).

**What increments CC** — `FunctionInfo.cyclomatic_complexity` starts at 1 (`lizard_core/function_info.py`); `lizard_core/file_analyzer.py::condition_counter` adds 1 for **every token** in `reader.conditions`, built from four sets in `lizard_languages/code_reader.py`: control flow `{if, for, while, catch}`, logical `{&&, ||}`, case `{case}`, ternary `{?}`. So `else`, `do`, `switch`, `default`, `try`, `finally`, `goto` are 0 by default.

- C/C++ (`lizard_languages/clike.py`): the default set, plus `preprocess()` calls `add_condition()` for `#if`, `#ifdef`, `#elif` (`#else`/`#endif`/`#define` are dropped; `#include` is re-emitted). "C/C++ preprocessing or macro expansion is not performed" (README). `template <…>` parameters are skipped. Rvalue-reference correction is partial (`CppRValueRefStates`): `&&` followed by `=` and `&&` inside a `typedef` are subtracted, but a `T&&` in a parameter list has no correction visible in the source — expect over-counting there. The pipeline order (preprocess → comment/line/token/condition counters → reader) means a `#if` before any function lands on the `*global*` pseudo-function.
- Python (`lizard_languages/python.py`): control flow `{if, elif, for, while, except, finally}`, logical `{and, or}`; `case` +1 when detected as a soft keyword; `match` +0 (only `-m` adds +1 and subtracts 1 per `case`). Hence `finally` +1, ternary `if` +1, comprehension `for` +1 and `if` +1, `case _` +1; `else`, `with`, `assert`, `lambda` +0.
- Rust (`lizard_languages/rust.py`): control flow `{if, for, while, catch, where}`, logical `{&&, ||}`, ternary `{?}` ("the error propagation operator"); `RustStates._state_global` adds 1 for each `=>` **after the first** inside a `match`; `let … else` +1 (`_count_let_else`); `loop` +0; closures +0. A generic `where` clause therefore adds 1.

**Other metrics** — NLOC per function: `line_counter` adds 1 per line that carries a non-comment token (function starts at 1). `length = end_line - start_line + 1` (physical, including comments and blanks). `parameter_count = len(parameters)`, where `full_parameters` is split on `,` and a regex strips defaults/annotations. `token_count`. `-END` gives `max_nesting_depth` (`lizard_ext/lizardnd.py`: nesting keywords are `if foreach for while && || ? catch case try def`; the first `&&`/`||` inside a condition adds a level, `else if` does not). `-ENS` counts nested control structures only. Whole-switch `-m` mode: `switch`/`match` +1, each `case` −1.

## crap4java (Java, for comparison)

`spec.md` only requires CC "computed from method bodies using Java syntax structure" with `CC >= 1` (§8.2) and `CRAP = CC^2 * (1 - coverage)^3 + CC`, threshold 8.0. The actual rules are `src/crap4java/JavaMethodParser.java::ComplexityCounter` (a javac `TreeScanner`): `complexity = 1`; +1 in `visitIf`, `visitForLoop`, `visitEnhancedForLoop`, `visitWhileLoop`, `visitDoWhileLoop`, `visitCatch`, `visitConditionalExpression`, `visitCase`, and `visitBinary` when the kind is `CONDITIONAL_AND`/`CONDITIONAL_OR`. `visitClass` returns `null`, so anonymous/nested class bodies are excluded; there is no lambda override, so lambda bodies fold into the enclosing method. `default` is a `CaseTree` in javac's API, so it counts. `switch`, `try`, `finally`, `else`, `break`: +0.

## Disagreement matrix

"+1" means one per occurrence. Columns: radon (Py), rca = rust-code-analysis (Py / Rs / C++), lizard (Py / Rs / C++), crap4java (Java).

| Construct | radon Py | rca Py | lizard Py | rca Rs | lizard Rs | rca C++ | lizard C++ | crap4java |
|---|---|---|---|---|---|---|---|---|
| Base per function | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| `if` / `elif` / `else if` | +1 | +1 | +1 | +1 | +1 | +1 | +1 | +1 |
| `else` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Loop `else` (Py) | +1 | +1 | 0 | – | – | – | – | – |
| `for` / `while` | +1 | +1 | +1 | +1 | +1 | +1 | +1 | +1 |
| `do…while` | – | – | – | – | – | +1 (via `while`) | +1 (via `while`) | +1 |
| `loop {}` (Rs) | – | – | – | +1 | 0 | – | – | – |
| Ternary | +1 | +1 | +1 | – | – | +1 | +1 | +1 |
| Comprehension clause | +1 (+1 per `if`) | +1 per `for`/`if` kw | +1 per `for`/`if` kw | – | – | – | – | – |
| `&&`/`||`/`and`/`or` | n−1 per chain | +1 each | +1 each | +1 each | +1 each | +1 each | +1 each (rvalue `&&` partly fixed) | +1 each |
| `switch` / `match` header | 0 | 0 | 0 (`-m`: +1) | 0 | 0 | 0 | 0 (`-m`: +1) | 0 |
| `case` / match arm | +1 (non-`_`) | 0 | +1 (incl. `_`) | +1 every arm | +1 per arm after first | +1 | +1 | +1 |
| `default` / `_` | 0 | 0 | +1 | +1 | +1 unless first | 0 | 0 | +1 |
| `try` | 0 | 0 | 0 | – | – | 0 | 0 | 0 |
| `except` / `catch` | +1 | +1 | +1 | – | – | +1 | +1 | +1 |
| `try…else` / `finally` | +1 / 0 | 0 / 0 | 0 / +1 | – | – | – / 0 | – / 0 | – / 0 |
| `with` | 0 (docs say +1) | +1 | 0 | – | – | – | – | – |
| `assert` | +1 (`--no-assert`) | +1 | 0 | – | – | 0 | 0 | 0 |
| `lambda` / closure | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 (body folds in) |
| `?` (Rs) | – | – | – | +1 | +1 | – | – | – |
| `if let` / `while let` | – | – | – | +1 | +1 | – | – | – |
| `let…else` | – | – | – | 0 | +1 | – | – | – |
| `where` clause | – | – | – | 0 | +1 | – | – | – |
| `#if` / `#ifdef` / `#elif` | – | – | – | – | – | 0 | +1 | – |
| `goto` | – | – | – | – | – | 0 | 0 | – |

Hot spots for slopmeter: Python `with`/`assert`/`finally`/`case _`; Rust `match` arms (all vs. after-first vs. none), `loop`, `where`; C++ `default`, `#if`, rvalue `&&`; and whether `and`/`or` chains are n−1 (all agree in effect) versus per-keyword.

Beyond CC, cross-checks for F04–F06: parameter count → lizard `PARAM` and rca `nargs`; nesting depth → lizard `-END` only (note its `&&`/`||`/`try` levels); function length → lizard `length`/`NLOC` and rca `end_line - start_line + 1`; LLOC → radon (file-level, colon heuristic) and rca `lloc` (statement nodes, per function).

## Sources

- https://radon.readthedocs.io/en/latest/intro.html — CC table and raw-metric definitions
- https://radon.readthedocs.io/en/latest/commandline.html — `radon cc` / `radon raw` flags
- https://raw.githubusercontent.com/rubik/radon/master/radon/visitors.py — `ComplexityVisitor` (the authority on increments)
- https://raw.githubusercontent.com/rubik/radon/master/radon/raw.py — LLOC algorithm
- https://raw.githubusercontent.com/rubik/radon/master/radon/complexity.py — `cc_rank`
- https://raw.githubusercontent.com/rubik/radon/master/radon/cli/tools.py — `cc_to_dict` JSON fields
- https://github.com/mozilla/rust-code-analysis — README (build instructions)
- https://mozilla.github.io/rust-code-analysis/ , https://mozilla.github.io/rust-code-analysis/languages.html , https://mozilla.github.io/rust-code-analysis/metrics.html , https://mozilla.github.io/rust-code-analysis/commands/metrics.html
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/metrics/cyclomatic.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/metrics/cognitive.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/metrics/nargs.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/metrics/loc.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/langs.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/src/spaces.rs
- https://raw.githubusercontent.com/mozilla/rust-code-analysis/master/rust-code-analysis-cli/src/main.rs
- https://github.com/terryyin/lizard — README
- https://raw.githubusercontent.com/terryyin/lizard/master/docs/cli.rst
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_languages/code_reader.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_languages/clike.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_languages/clike_states.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_languages/python.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_languages/rust.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_core/file_analyzer.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_core/file_info_builder.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_core/function_info.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_core/options.py
- https://raw.githubusercontent.com/terryyin/lizard/master/lizard_ext/csvoutput.py , …/lizard_ext/xmloutput.py , …/lizard_ext/lizardnd.py , …/lizard_ext/lizardns.py , …/lizard_ext/lizardcpre.py , …/lizard_ext/lizardmodified.py
- https://raw.githubusercontent.com/unclebob/crap4java/main/spec.md
- https://raw.githubusercontent.com/unclebob/crap4java/main/src/crap4java/JavaMethodParser.java
- https://api.github.com/repos/{rubik/radon,mozilla/rust-code-analysis,terryyin/lizard,unclebob/crap4java} — archive status / last push; https://pypi.org/pypi/radon/json , https://pypi.org/pypi/lizard/json , https://crates.io/api/v1/crates/rust-code-analysis-cli — versions
