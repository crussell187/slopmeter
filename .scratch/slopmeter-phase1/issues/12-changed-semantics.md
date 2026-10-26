# What --changed means

Type: grilling
Status: resolved
Blocked by: 11

## Question

Given the diff-scoping decision, what does `--changed` compare against: the working tree vs HEAD via `git status --porcelain` (crap4java's choice, right for a pre-commit loop), or a diff against a base ref (`--changed=main`, right for CI and for an agent working on a branch)? Is function-level change detection by line range intersection with the diff hunks, and what happens to a function whose body did not change but whose file did? What does the tool do outside a git repo?

## Answer

Resolved 2026-10-26 with Christopher. Why the diff is the branch plus the working tree: [ADR 0008](../../../docs/adr/0008-what-changed-means.md). The glossary term is **Touched function** in `CONTEXT.md`.

The default diff runs from the merge-base of `HEAD` and `main` through the working tree. Committed work on the branch and uncommitted edits both count. `--changed=<ref>` picks another base. `--changed=HEAD` is only the changes since the last commit. On `main` itself, only uncommitted changes count. `--strict` scores the whole repo.

A function is touched when a changed line hits its span, from the first decorator or attribute through the end of the body. A change to another function in the same file does not touch it. A blank line above the decorator does not either. A pure rename with no changed line inside the span does not either. Decorator and attribute lines still add nothing to length or complexity.

If `main` is missing, the base is `master`. If neither exists, or the merge-base cannot be computed, the run fails and says to pass `--changed=<ref>` or `--strict`. A directory that is not a git repo is scored in full, and the run says so. An empty diff gates nothing.
