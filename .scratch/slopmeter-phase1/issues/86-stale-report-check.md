# Treat a report older than the change as missing

Type: task
Status: open
Blocked by: 

## Question

Build [ADR 0035](../../../docs/adr/0035-a-report-older-than-the-change-is-missing.md). `check` reads the modification time of each report `inputs::lcov` and `inputs::mutation` were given, and on the default run compares it to every path in the diff's change set, untracked files included; one newer path marks the report stale, each touched function fails S06 or S07 on its function line, CRAP is absent for the report's files, and one note names the report and the newest file. Under `--strict` and outside git the comparison is per scored file the report matches, and a newer file gets the note and no row. For mutmut the per-file `.meta` time is that file's report time and the newest `.meta` is the store's time. A tie is fresh. Tests set modification times explicitly rather than sleeping.
