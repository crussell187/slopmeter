# Pin the diff header shape and resolve the repo root

Type: task
Status: open
Blocked by: 

## Question

With repo-local `diff.noprefix=true` or `diff.mnemonicPrefix=true`, the `a/`/`b/` header parse in `src/scope.rs:420-431` matches nothing, so no function is touched and a 96-path function passes as inherited (review finding 3, verified). Pass `--src-prefix=a/ --dst-prefix=b/` (or `-c diff.noprefix=false -c diff.mnemonicPrefix=false`) at `src/scope.rs:265-280` and add a test with that config. In the same file, replace the ceiling search at `:244-257` and `:489-492` with `git rev-parse --show-toplevel` and `--relative` paths so a run from a subdirectory or on a subdirectory path scopes correctly and reads the root `slopmeter.toml` and `.gitignore`; fall back to `origin/main` then `origin/master` when no local base branch exists (review finding 9).
