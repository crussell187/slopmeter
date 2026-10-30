---
status: accepted
date: 2026-10-29
---

# A run scores every claimed extension

A run scores every claimed file, and the file's extension picks the profile. The percentile for a profile is the files of that language in the three repos pinned for that language. A file of another language in those repos is scored, and it stays out of every percentile. The integers measured on 2026-10-29 stay. A project narrows the languages in a run with `exclude`.

## Considered options

- **Score only the language the repo is pinned as.** Rejected. A pin's language is how the nine repos were chosen. A tree someone points the tool at has no such language, and a clone of fmt would then score differently from the validation run of the same commit.
- **Fold every file of a language across the nine pins into that language's percentile.** Rejected. `support/docopt.py` in fmt is a copied parser, and letting it move the Python gates would teach those gates a file that is not part of the Python corpus. The same applies to `regex-capi/include/rure.h` and the C++ percentile.
- **A language list in the config.** Rejected. `include` and `exclude` are the file keys. A second departure would shrink the scored set a second way.

## Consequences

- [Count gates are a percentile of the pinned clean repos](0031-count-gates-are-a-percentile.md) keeps its integers. A later measurement leaves a file of another language out of every percentile.
- A `--strict` run of fmt, re2, and CLI11 still scores their `.py` files under the Python profile. A function over that gate fails. That is the tail a clean repo already shows.
- A `--strict` run of regex scores `regex-capi/include/rure.h` under the C++ profile. That header is in neither percentile.
- [The default scored set leaves tests out](0005-scored-set.md) still names which claimed files a run measures. This decision adds no second filter.
