# PROTOTYPE — wipe me

Question: can Phase 1 score one real file per language from a language-neutral list of functions, without module edges, classes, churn, or coverage stored on the IR?

```bash
python3 prototype/ir-schema/show.py
```

`n` / `p` switch file. `j` / `k` switch function. `q` quits.

Hand-encoded, not parsed, from `psf/requests` `src/requests/api.py`, `tokio-rs/bytes` `src/buf/take.rs`, and `CLIUtils/CLI11` `include/CLI/StringTools.hpp`. Not the crate.

The decision dropped this sample's `version` field. The Rust types are the schema, and the document stays in memory. See [ADR 0015](../../docs/adr/0015-ir-is-files-and-functions.md).
