---
status: accepted
date: 2026-10-26
---

# Rust for the slopmeter core

The machine this was charted on has Node 24 and Python 3.9 but no Rust toolchain, and the user's other projects are Python, so Rust is not the obvious path. We chose Rust anyway, as the spec reasons: a single static binary that every agent sandbox can run without a venv or node_modules, parse speed on large repos, tree-sitter's native home, and Phase 3 dogfooding (the tool checks its own Rust). Python was rejected for distribution and parse speed; TypeScript was considered during charting (toolchain already present, npm distribution) and rejected because it would trade the single-binary goal for a faster first week and likely force a port later.

## Consequences

- Installing rustup is a task on the map before any build slice.
- Each language grammar is a tree-sitter crate behind the adapter seam.
