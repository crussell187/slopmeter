# Pass-through and Demeter detection without symbol resolution

Type: grilling
Status: open
Blocked by: 06

## Question

F07 flags a single-statement body forwarding ≥ 80% of its params to one callee; the spec's risk section says it must exempt classes implementing an interface with ≥ 2 implementations, which needs symbol resolution Phase 1 does not have. F08 flags member-access chains deeper than the threshold, excluding fluent builders and stdlib collections via an allow-list. For Python, Rust and C++: which exemptions can Phase 1 honour purely syntactically (trait impls in Rust, `@abstractmethod` overrides in Python, `override` in C++), what goes in the default allow-lists, and do F07/F08 ship as gates, warns, or off for any language in Phase 1 given the false-positive risk?
