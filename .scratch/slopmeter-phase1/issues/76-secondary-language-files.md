# Whether a run scores every claimed extension

Type: grilling
Status: open
Blocked by:

## Question

A `--strict` run of the pinned C++ repos still scores their `.py` files under the Python profile: 41 gate findings in fmt, 20 in re2, and 3 in CLI11, on the 2026-10-29 measurement. [Count gates are a percentile of the pinned clean repos](../../../docs/adr/0031-count-gates-are-a-percentile.md) left those files out of both languages' percentiles. Does a run score every claimed extension in the tree, or only the language the repo is pinned as?
