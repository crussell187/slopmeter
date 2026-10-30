# Slopmeter

A deterministic quality gate that scores source code against software-design metrics and returns machine-readable findings an LLM coding agent must act on before its change is accepted.

## Language

**Slop**:
Code that passes its tests but is structurally poor: sprawling functions, pass-through modules, pervasive coupling, duplication, complex-and-untested logic.
_Avoid_: bad code, tech debt, smell (a smell is one symptom; slop is the condition)

**Rule**:
One named, numbered check (F01, M04, S01…) that compares a metric against a threshold. Belongs to exactly one scope.
_Avoid_: check, lint, policy

**Scope**:
The unit a rule measures: function, module (file, class or package), system (whole repo or diff), or architecture.

**Scored set**:
The claimed files in the directory a run starts in. A package boundary does not split it, and tests are out unless a run turns them on.
_Avoid_: codebase, sources, package

**Function**:
A named definition with a body the programmer wrote, including one nested inside another, or a lambda or closure bound to a name outside any function. Declarations, compiler-generated bodies, decorators, and attributes sit outside it; a generic or template counts once, and any other lambda or closure belongs to the function that contains it. The contents of a macro invocation written inside a function belong to that function. A macro invocation outside any function is not a function. A Rust macro definition written outside any function is itself a function; one written inside a function belongs to that function. A C++ `#define` is not a function.
_Avoid_: method, lambda, closure, callable, declaration, prototype, decorator, attribute

**Body span**:
The lines from the `def` or `fn` through the end of the body. Coverage and the mutation score join on this span. A mutant belongs to the function whose span contains the line where that mutant starts.
_Avoid_: touch span, extent

**Touch span**:
The body span extended up through a leading decorator, non-doc attribute, and the contiguous waiver lines directly above them. A doc comment does not extend it, and a changed line here touches the function.
_Avoid_: body span, dirty range

**Pass-through**:
A function whose body hands its parameters, other than the receiver, unchanged to one call, as an argument, as that call's object, or as a pack expansion of that hand-off, alongside only literal arguments. A logging statement, an assignment to a name that is never read, and a wrapping `Ok`, `Some`, or `Err` do not change that.
_Avoid_: wrapper, delegate, pass-through module

**Member chain**:
Consecutive `.` or `->` accesses in one expression, so a local that holds an earlier chain starts a new one. A chain from the receiver may be two deep, and any other chain may be one deep.
_Avoid_: train wreck, dot chain

**Logical line**:
One statement in a function's body, including a closure's statements and a Rust tail expression. The signature, a blank, a comment, a docstring, a brace, and a decorator or attribute are not one.
_Avoid_: physical line, source line, LOC

**Function length**:
The physical lines of the body span, from the `def` or `fn` line through the end of the body. A decorator or attribute is outside it.
_Avoid_: logical line, NLOC

**Parameter count**:
The ordinary parameters of a function, including one with a default and a varargs or keyword pack. The receiver is not one.
_Avoid_: arity

**Cyclomatic complexity**:
The number of independent paths through a function. It starts at 1, and each decision point adds 1.
_Avoid_: cognitive complexity, complexity

**Nesting**:
The deepest structural level in a function: an `if`, a loop, a `match` or `switch`, a ternary, and a `catch` each add one inside the body or a ternary's value operand, handlers on one `try` share that level, and a function with none of these has nesting 0. A condition, an `else`, an `elif`, an `else if`, a boolean operator, a lambda, a guard, a comprehension clause, a `try`, a `finally`, a `with`, and a `#if` add none.
_Avoid_: cognitive depth, indent, decision depth

**Cognitive complexity**:
The cost of understanding a function. A nested break adds 1 plus its depth. An `else`, a boolean sequence, or a labelled jump adds 1. The same table applies in Python, Rust, and C++.
_Avoid_: cyclomatic complexity, complexity

**Coverage**:
The fraction of a function's executable lines that the report marks hit, joined on the body span. A line is executable when the report lists it. With no listed lines, coverage is absent and CRAP is not computed.
_Avoid_: changed-line coverage, branch coverage, instruction coverage

**CRAP**:
A function metric that grows with cyclomatic complexity and with uncovered lines. The score is CC squared times the uncovered fraction cubed, plus CC. Absent coverage leaves it uncomputed.
_Avoid_: crap score, grade

**Mutation score**:
The fraction of a function's valid mutants that the tests detected. A mutant is detected when a test fails on it or the run times out. A mutant the tests pass is undetected. A mutant that never compiles is left out. With no valid mutants, the score is absent.
_Avoid_: kill rate, coverage, CRAP

**Unparsed ratio**:
The fraction of lines in a file that the parser could not recover. It says how far to trust that file's numbers, and it does not fail the run.
_Avoid_: error rate, parse failure

**Tier**:
How trustworthy a rule's signal is: A is deterministic and gate-worthy, B is deterministic but noisy and warns by default, C needs semantics the tool lacks and is not computed.

**Gate**:
A finding at gate level in a touched function. It fails the run. A waiver on that finding keeps it a gate, and the verdict ignores it unless the change under review added the waiver line. A lower warn level on the same rule does not.
_Avoid_: error, blocker, hard fail

**Waiver**:
A stated measurement and a reason that excuse one gate on one function. The finding stays a gate in the report, and the verdict ignores it while the measurement is at most that number.
_Avoid_: suppression, noqa, allow

**Warn**:
A finding in the report and in the score that does not fail the run.
_Avoid_: warning, soft fail, notice

**Finding**:
The worst violation of one rule at one function: rule, symbol, the line of that site, measured value, the threshold it is over, level (`gate` or `warn`), weight, principle, fix shape, anti-fix. A waived gate also carries the excused measurement and the reason.
_Avoid_: issue, violation, diagnostic, error

**Inherited finding**:
A finding in a function the change did not touch. It is listed, and it does not gate.
_Avoid_: legacy, baseline

**Touched function**:
A function whose touch span contains a changed line.
_Avoid_: modified file, dirty file

**Slop score**:
Every finding's weight, inherited findings included, divided by the thousands of physical lines in the scored set. The same tree scores the same on a diff run and on `--strict`. Reported beside the verdict, and it does not fail the run. Absent when the scored set has no lines.
_Avoid_: quality score, grade, ceiling

**Principle**:
The one-sentence design rule, with its source, that a finding is a proxy for.

**Fix shape**:
The named refactoring a finding suggests, filled with the measured value, the threshold, and the function's location. It does not predict the result of the edit. Never contains code.
_Avoid_: fix, suggestion, autofix

**Anti-fix**:
The cheapest way to make a finding's number go green that the tool still catches; stated so the agent knows the shortcut will not pass.
_Avoid_: counter-rule (that is the rule that catches it), gaming

**Adapter**:
The per-language stage that turns one selected file into that file's IR. It publishes that language's default allow-list. It does not see other files, the coverage report, or names a project adds.
_Avoid_: plugin, frontend, parser (a parser is a part of it)

**IR**:
The single language-neutral intermediate representation every stage after the adapters shares. In Phase 1 it is the scored files and their functions. Nothing below the IR sees source text.
_Avoid_: AST, model

**Analyzer**:
A pure function from IR to raw measurements. Knows no thresholds.
_Avoid_: checker, metric (a metric is what it produces)

**Rule engine**:
The one stage that knows thresholds, weights, waivers, and what "bad" means; turns measurements into findings and the verdict.

**Reporter**:
A stage that renders findings: JSON, worst-first text table, exit code.

**Explain**:
The one-page description of one rule: its principle, its counting method, one threshold table of the shipped numbers, one before and one after, and the anti-fix list. A gate's page also states how that gate is waived.
_Avoid_: help text, man page

**Allow-list**:
Member names that add no hop on a member chain. The adapter publishes the language's defaults. A project adds single member names.
_Avoid_: exemption, ignore list

**Profile**:
One language's thresholds. A count gate and the cognitive warn are that language's recorded percentile of its clean repos, replaced only by a later measurement. The percentile is taken from the files of that language in the repos pinned for it. The tool ships the table, including the pass-through ratio and the two chain depths. The config file may change a gate on a count rule, a warn only on cognitive complexity, and names on that language's allow-list. The coverage gate and the mutation gate are not in the profile.

**Departure**:
A profile value set to something other than that language's built-in. The run names it, and it does not fail the run.
_Avoid_: override, customization

**Validation run**:
A `--strict` score of the per-language corpus once thresholds have been tuned. The clean repos show the functions over those gates. The sloppy repo fails each gating rule by a wide margin, and shows the F07 warn.
_Avoid_: acceptance test (that is the separate with-tool vs without-tool agent experiment, out of scope for Phase 1)

**Sloppy repo**:
A pinned tree, one per language, that an agent wrote from a fixed seed. It is kept when every gating rule fires by a wide margin, and the function the tests never call is left unhit in its coverage report.
_Avoid_: fixture, acceptance test
