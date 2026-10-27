# PROTOTYPE — wipe me

Question: can a mixed Python and Rust repo say what it needs in `slopmeter.toml` by writing only the keys it changes?

Answer: yes. [ADR 0018](../../docs/adr/0018-config-records-departures.md).

```bash
cat prototype/config-shape/slopmeter.toml prototype/config-shape/narrow.toml
```

`slopmeter.toml` is the file most repos would commit. `narrow.toml` is the same shape when a repo also narrows the scored set and adds one allow-list name. Numbers here are one repo's overrides. They are not the built-in defaults.
