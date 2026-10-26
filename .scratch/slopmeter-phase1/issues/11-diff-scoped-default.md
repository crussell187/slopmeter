# Diff-scoped by default, inherited findings, and --strict

Type: grilling
Status: open
Blocked by: 

## Question

The spec's open question: is the default run diff-scoped (findings only in touched functions, pre-existing violations listed under `inherited` and never gating) or whole-repo? The spec leans diff-scoped with a `--strict` flag. Decide the default, what `inherited` means for a function that was already over threshold and got worse (does worsening gate?), and whether a first run with no baseline is whole-repo.
