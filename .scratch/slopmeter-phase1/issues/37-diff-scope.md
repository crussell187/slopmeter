# Gate touched functions

Type: task
Status: resolved
Blocked by: 36

## Question

Implement session 10 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). The default run puts touched findings in `findings` and the rest in `inherited`. `--strict` scores the whole set. An empty diff gates nothing.

## Answer

`scope::partition` joins the diff to each function's touch span. A changed line, including a deleted line that sat in the span at the base, puts every finding for that function in `findings`. The rest stay in `inherited`. `--strict` scores the whole set and does not need a base. An empty diff gates nothing. No `main` or `master` fails the run and asks for `--changed=<ref>` or `--strict`. A directory that is not a git repo is scored in full, and the run says so. The binary does not score a tree yet. The code-level plan is [Diff scope](../../../docs/superpowers/plans/2026-10-27-diff-scope.md).
