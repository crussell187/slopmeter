# Treat a report older than the change as missing

Type: task
Status: resolved
Blocked by: 

## Question

Build [ADR 0035](../../../docs/adr/0035-a-report-older-than-the-change-is-missing.md). `check` reads the modification time of each report `inputs::lcov` and `inputs::mutation` were given, and on the default run compares it to every path in the diff's change set, untracked files included; one newer path marks the report stale, each touched function fails S06 or S07 on its function line, CRAP is absent for the report's files, and one note names the report and the newest file. Under `--strict` and outside git the comparison is per scored file the report matches, and a newer file gets the note and no row. For mutmut the per-file `.meta` time is that file's report time and the newest `.meta` is the store's time. A tie is fresh. Tests set modification times explicitly rather than sleeping.

## Answer

Resolved 2026-10-30. On the default run a coverage or mutation report older than any path in the diff, untracked files included, is missing: each touched function fails S06 or S07 on its function line, CRAP is absent, and one note names the report and the newest path. A deleted path has no modification time and counts as newer; among those, the note names the greatest path. A tie is fresh. Under `--strict` and outside git the comparison is per scored file the report matches: a newer file gets that note and no row, and an older file keeps its rows. Mutmut uses each `mutants/<file>.meta` time for that file, and the newest `.meta` as the store's time against the diff. The report's own path, and anything under a directory report, stays out of the comparison. `slopmeter --strict` on this repo still exits 0.
