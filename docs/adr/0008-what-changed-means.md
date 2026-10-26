---
status: accepted
date: 2026-10-26
---

# The change is the branch plus the working tree

The default run gates functions the change touched, so the diff has to include every commit on the branch, not only the uncommitted work. It runs from the merge-base of `HEAD` and `main` through the working tree. `--changed=<ref>` picks another base. `--changed=HEAD` is only what is new since the last commit. On `main` itself, only uncommitted changes count.

A function is touched when a changed line hits its span, from the first decorator or attribute through the end of the body. A change elsewhere in the same file does not touch it. Those decorator lines still add nothing to the function's measurements.

If `main` is missing, the base is `master`. If neither exists, or the merge-base cannot be computed, the run fails and asks for `--changed=<ref>` or `--strict`. A directory that is not a git repo is scored in full, and the run says so. An empty diff still gates nothing.

## Considered options

- **Working tree against `HEAD` only.** Rejected. A failing function would pass on the next clean run after it was committed.
- **Outside a git repo, gate nothing.** Rejected. That is an empty diff, and this case is not one: the tool cannot tell whether anything changed.

## Consequences

- `--strict` remains the whole-repo run.
- A pure rename with no changed line inside the span does not touch the function.
