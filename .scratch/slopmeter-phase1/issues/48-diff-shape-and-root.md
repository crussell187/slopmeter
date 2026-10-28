# Pin the diff header shape and resolve the repo root

Type: task
Status: resolved
Blocked by: 

## Question

With repo-local `diff.noprefix=true` or `diff.mnemonicPrefix=true`, the `a/`/`b/` header parse in `src/scope.rs:420-431` matches nothing, so no function is touched and a 96-path function passes as inherited (review finding 3, verified). Pass `--src-prefix=a/ --dst-prefix=b/` (or `-c diff.noprefix=false -c diff.mnemonicPrefix=false`) at `src/scope.rs:265-280` and add a test with that config. In the same file, replace the ceiling search at `:244-257` and `:489-492` with `git rev-parse --show-toplevel` and `--relative` paths so a run from a subdirectory or on a subdirectory path scopes correctly and reads the root `slopmeter.toml` and `.gitignore`; fall back to `origin/main` then `origin/master` when no local base branch exists (review finding 9).

## Answer

`git diff` forces `a/` and `b/` and turns `diff.noprefix` and `diff.mnemonicPrefix` off, so either repo-local setting still touches the edited function. `scope::repo_root` is `git rev-parse --show-toplevel` with no ceiling directory. A subdirectory is that work tree: `--relative` keeps the diff lined up with the directory, `git status` paths are stripped by `--show-prefix`, and a deletion is read with that prefix so only the function it hits is touched. `check::score` reads `slopmeter.toml` from the git root and ignores a nested file. Include and exclude stay anchored at the root; the report path stays relative to the directory asked for. Parent `.gitignore` rules apply inside a repo and stay off outside one. The base is local `main`, then `master`, then `origin/main`, then `origin/master`. The code-level plan is [Diff shape and repo root](../../../docs/superpowers/plans/2026-10-27-diff-shape-and-root.md).
