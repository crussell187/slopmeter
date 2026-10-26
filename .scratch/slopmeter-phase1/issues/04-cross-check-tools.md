# How radon, rust-code-analysis and lizard count cyclomatic complexity

Type: research
Status: resolved
Context: research subagent fired 2026-10-26; findings land on branch `research/cross-check-tools` at `docs/research/cross-check-tools.md`
Blocked by: 

## Question

To validate slopmeter's numbers against an independent ruler per language, how do the established tools count cyclomatic complexity, and what else do they measure? Python: radon (`radon cc`, `radon raw`) — its CC rules (comprehensions, boolean operators, `with`, `assert`, lambdas), LLOC definition, and output format. Rust: Mozilla's rust-code-analysis — CC and cognitive complexity rules, output format, whether it also covers Python and C++. C++ (and multi-language): lizard — CC rules, LLOC, parameter counts, nesting, output format, and its C++ preprocessor handling. Also note any of these that compute parameter count, nesting depth or function length, since slopmeter's F04–F06 can be cross-checked too. For each tool: install command, how to run it on a directory and get machine-readable output, and the exact source of its counting rules (code or docs). Flag where the tools disagree with each other and with crap4java's rules (+1 per if, loop, catch, ?:, switch label, &&, ||; base 1).

## Answer

Resolved 2026-10-26 by a research subagent. Full findings: `docs/research/cross-check-tools.md` on branch `research/cross-check-tools` (read with `git show research/cross-check-tools:docs/research/cross-check-tools.md`).

- **Core agreement across radon, rust-code-analysis, lizard and crap4java:** base 1 per function; +1 per `if`/`elif`, loop, `catch`/`except`, ternary, and each `&&`/`||` (radon's per-`BoolOp` `len(values)-1` is numerically the same). `else`, `try`, `switch`/`match` headers, lambdas and closures are 0 everywhere.
- **Python disagreements:** `with` is 0 in radon (docs say +1, code has no `With` branch), +1 in rust-code-analysis, 0 in lizard. `assert` +1 only in rust-code-analysis. `finally` +1 only in lizard. Loop `else` +1 in radon and rust-code-analysis, 0 in lizard. `match`: radon counts non-wildcard cases, lizard counts every case including `_`, rust-code-analysis ignores `match`.
- **Rust disagreements:** rust-code-analysis counts every match arm including the first and `_`; lizard counts arms after the first, so a 3-arm match is +3 vs +2. rust-code-analysis counts `loop {}` +1; lizard counts a generic `where` clause +1 and `let…else` +1. Both count `?`, `if let`, `while let` +1.
- **C++:** both count `case` but not `default`; crap4java (Java) counts `default` because javac models it as a `CaseTree`. lizard adds +1 per `#if/#ifdef/#elif` without macro expansion (`-Ecpre` drops `#else` branches) and only partially corrects rvalue-reference `&&`; rust-code-analysis counts neither preprocessor nor `goto`.
- **crap4java's spec.md does not define CC rules;** they live in `JavaMethodParser.ComplexityCounter` and match the ticket's summary exactly. Nested/anonymous classes skipped; lambda bodies fold into the enclosing method.
- **Aggregation gotchas:** radon's class complexity is an average over methods; rust-code-analysis `cyclomatic.sum/average` include the file-level unit (+1); lizard attributes top-level code to a `*global*` pseudo-function.
- **F04–F06 cross-checks:** parameter count → lizard `PARAM`, rust-code-analysis `nargs`; nesting depth → only lizard `-END` (adds a level for the first `&&`/`||` and for `try`/`def`); function length → lizard `length`/`NLOC`, rust-code-analysis `end_line - start_line + 1`; LLOC → radon (file-level colon heuristic, over-counts dict literals) and rust-code-analysis `lloc` per function.
- **Running them:** `pip install radon`, `pip install lizard` are current; rust-code-analysis must be built from source (`cargo build -p rust-code-analysis-cli`, crates.io CLI is from 2023). Machine-readable: `radon cc -j`, `radon raw -j`, `lizard --csv`/`--xml`, `rust-code-analysis-cli -m -p <dir> -O json -o <out>`.
- **Caveat:** rust-code-analysis matches keyword tokens, so Python comprehension `for`/`if` and ternary `if` almost certainly count, inferred from the grammar rather than covered by its tests.
- **Implication for ticket 07:** no single tool can be the parity target for all three languages; the doc's disagreement matrix is the input for choosing per-language parity or a documented bounded delta.
