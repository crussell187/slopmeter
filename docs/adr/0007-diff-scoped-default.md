---
status: accepted
date: 2026-10-26
---

# A touched function has to pass

The default run is for an agent's change, not for the history of the repo. Findings in functions the change did not touch are inherited: listed, and they do not gate. `--strict` scores the whole repo. Once a function is touched, it has to come in under its gate level. A value that got worse, held, or improved and is still over the line all gate. A small edit on top of a failing function is not accepted. The baseline is the diff, not a stored slopmeter result. An empty diff gates nothing.

## Considered options

- **A pre-existing finding gates only when the change made it worse.** Rejected. A typo fix could land inside a function that still fails, so an update to bad code would not be forced clean.
- **The first run with no stored result scores the whole repo.** Rejected. Cloning a repo and running the tool would fail the build on code the checkout did not change. `--strict` is that run.

## Consequences

- The glossary term **inherited finding** means a finding in a function the change did not touch.
- A warn below the gate level stays a warn.
- How a function counts as touched, and what happens when git cannot produce a diff, stay on [What --changed means](../../.scratch/slopmeter-phase1/issues/12-changed-semantics.md).
