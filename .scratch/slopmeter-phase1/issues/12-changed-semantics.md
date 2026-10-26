# What --changed means

Type: grilling
Status: open
Blocked by: 11

## Question

Given the diff-scoping decision, what does `--changed` compare against: the working tree vs HEAD via `git status --porcelain` (crap4java's choice, right for a pre-commit loop), or a diff against a base ref (`--changed=main`, right for CI and for an agent working on a branch)? Is function-level change detection by line range intersection with the diff hunks, and what happens to a function whose body did not change but whose file did? What does the tool do outside a git repo?
