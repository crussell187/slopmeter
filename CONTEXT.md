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

**Tier**:
How trustworthy a rule's signal is: A is deterministic and gate-worthy, B is deterministic but noisy and warns by default, C needs semantics the tool lacks and is not computed.

**Gate**:
A rule outcome that fails the run (non-zero exit). Gates are deliberately few.
_Avoid_: error, blocker, hard fail

**Warn**:
A rule outcome that appears in the report with a score contribution but does not fail the run.
_Avoid_: warning, soft fail, notice

**Finding**:
One rule violation at one symbol: rule, symbol, location, measured value, threshold, weight, principle, fix shape.
_Avoid_: issue, violation, diagnostic, error

**Inherited finding**:
A finding that pre-existed the change under review and was not worsened by it. Listed, never gating.

**Slop score**:
Σ finding weights ÷ KLOC for the scored set, so repos of different size compare. Reported alongside the gate verdict.
_Avoid_: quality score, grade

**Principle**:
The one-sentence design rule, with its source, that a finding is a proxy for.

**Fix shape**:
The named refactoring a finding suggests (extract, inline, introduce parameter object, invert dependency, merge modules), filled with the measured numbers. Never contains code.
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
