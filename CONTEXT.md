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
The claimed files a run measures. Tests are out of it unless a run turns them on.
_Avoid_: codebase, sources

**Function**:
A named definition with a body the programmer wrote, including one nested inside another, or a lambda or closure bound to a name outside any function. Declarations, compiler-generated bodies, decorators, and attributes sit outside it; a generic or template counts once, and any other lambda or closure belongs to the function that contains it.
_Avoid_: method, lambda, closure, callable, declaration, prototype, decorator, attribute

**Pass-through**:
A function whose body only hands its parameters, other than the receiver, unchanged to one call.
_Avoid_: wrapper, delegate, pass-through module

**Member chain**:
Consecutive `.` or `->` accesses in one expression. A chain that starts at the receiver may be two deep; any other chain may be one deep.
_Avoid_: train wreck, dot chain

**Cyclomatic complexity**:
The number of independent paths through a function. It starts at 1, and each decision point adds 1.
_Avoid_: cognitive complexity, complexity

**Tier**:
How trustworthy a rule's signal is: A is deterministic and gate-worthy, B is deterministic but noisy and warns by default, C needs semantics the tool lacks and is not computed.

**Gate**:
A finding at gate level in a touched function. It fails the run. A lower warn level on the same rule does not.
_Avoid_: error, blocker, hard fail

**Warn**:
A finding in the report and in the score that does not fail the run.
_Avoid_: warning, soft fail, notice

**Finding**:
The worst violation of one rule at one function: rule, symbol, the line of that site, measured value, the threshold it is over, level (`gate` or `warn`), weight, principle, fix shape, anti-fix.
_Avoid_: issue, violation, diagnostic, error

**Inherited finding**:
A finding in a function the change did not touch. It is listed, and it does not gate.
_Avoid_: legacy, baseline

**Touched function**:
A function whose span contains a changed line. The span runs from the first decorator or attribute above the definition through the end of the body.
_Avoid_: modified file, dirty file

**Slop score**:
Σ finding weights ÷ KLOC for the scored set, so repos of different size compare. Reported beside the verdict, and it does not fail the run.
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
The per-language stage that parses source and produces IR, and nothing else.
_Avoid_: plugin, frontend, parser (a parser is a part of it)

**IR**:
The single language-neutral intermediate representation every stage after the adapters shares. Nothing below the IR sees source text.
_Avoid_: AST, model

**Analyzer**:
A pure function from IR to raw measurements. Knows no thresholds.
_Avoid_: checker, metric (a metric is what it produces)

**Rule engine**:
The one stage that knows thresholds, weights and what "bad" means; turns measurements into findings and the verdict.

**Reporter**:
A stage that renders findings: JSON, worst-first text table, exit code.

**Profile**:
The per-language table of default thresholds and rule settings in `slopmeter.toml`.

**Validation run**:
Scoring the per-language corpus: clean open-source repos must pass, the deliberately sloppy agent-written repo must fail on every rule it should.
_Avoid_: acceptance test (that is the separate with-tool vs without-tool agent experiment, out of scope for Phase 1)
