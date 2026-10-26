# Diff-scoped by default, inherited findings, and --strict

Type: grilling
Status: resolved
Blocked by: 

## Question

The spec's open question: is the default run diff-scoped (findings only in touched functions, pre-existing violations listed under `inherited` and never gating) or whole-repo? The spec leans diff-scoped with a `--strict` flag. Decide the default, what `inherited` means for a function that was already over threshold and got worse (does worsening gate?), and whether a first run with no baseline is whole-repo.

## Answer

Resolved 2026-10-26 with Christopher. Why a touched function has to pass: [ADR 0007](../../../docs/adr/0007-diff-scoped-default.md). The glossary term **inherited finding** now means a finding in a function the change did not touch.

The default run gates only findings in functions the change touched. `--strict` scores the whole repo. An inherited finding is listed and does not gate. If the change touched the function and it is still over its gate level, the finding gates, whether the value got worse, held, or improved. A function that was 40 and is now 16 still fails a gate of 15. A warn below the gate level stays a warn.

The baseline is the diff, not a stored slopmeter result. A clean checkout gates nothing. A branch with commits, or a dirty working tree, gates every touched function that is still over its line.

How a function counts as touched, and what happens when git cannot produce a diff, stay on [What --changed means](12-changed-semantics.md).
