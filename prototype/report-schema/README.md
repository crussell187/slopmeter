# PROTOTYPE — wipe me

Question: can an agent tell, from one JSON report, which finding it must fix for the run to pass?

```bash
python3 -m json.tool prototype/report-schema/report.json
```

Hand-written. Not a parser output. The principle and the anti-fix are stand-in text. `slop_score` is `(10 + 1 + 10) / 0.04`, the weights in both lists over the stand-in size. `tool_version` is a stand-in. The note is the run-level coverage gap.
