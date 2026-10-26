# How radon, rust-code-analysis and lizard count cyclomatic complexity

Type: research
Status: open
Blocked by: 

## Question

To validate slopmeter's numbers against an independent ruler per language, how do the established tools count cyclomatic complexity, and what else do they measure? Python: radon (`radon cc`, `radon raw`) — its CC rules (comprehensions, boolean operators, `with`, `assert`, lambdas), LLOC definition, and output format. Rust: Mozilla's rust-code-analysis — CC and cognitive complexity rules, output format, whether it also covers Python and C++. C++ (and multi-language): lizard — CC rules, LLOC, parameter counts, nesting, output format, and its C++ preprocessor handling. Also note any of these that compute parameter count, nesting depth or function length, since slopmeter's F04–F06 can be cross-checked too. For each tool: install command, how to run it on a directory and get machine-readable output, and the exact source of its counting rules (code or docs). Flag where the tools disagree with each other and with crap4java's rules (+1 per if, loop, catch, ?:, switch label, &&, ||; base 1).
