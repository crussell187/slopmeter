# Repo, crate layout, CI and where the GitHub repo lives

Type: grilling
Status: open
Blocked by: 

## Question

This directory is now a git repo with no remote. Does slopmeter get a GitHub repo (which account, public or private), and does the map migrate from local markdown to GitHub Issues then (needs the `gh` CLI installed)? Is the Rust project a single crate with modules per pipeline stage, or a Cargo workspace with one crate per stage (inputs, adapters, ir, analyzers, rules, reporters) plus the CLI? Where do language grammars live (one adapter crate per language)? What CI runs on push: fmt, clippy, tests, and eventually the self-check? Where do research notes and ADRs live (currently `docs/research/` and `docs/adr/`)?
