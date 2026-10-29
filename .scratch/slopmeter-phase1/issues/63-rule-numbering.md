# Reconcile rule ids with the spec and the dropped commented-out-code rule

Type: grilling
Status: resolved
Blocked by: 

## Question

The spec numbered F04 logical lines, F05 parameters, F06 nesting and F11 commented-out code; the build ships F04 parameters, F05 nesting, F06 physical length and F11 logical lines, and no rule, explain page or ADR covers commented-out code (review, spec drift). The design doc, the explain pages and any agent prompt written from the spec now disagree with the tool. Decide: keep the build's ids and record the mapping and the dropped rule in an ADR, or renumber to the spec and implement commented-out code (a comment block that parses as code in the host language) as F11. Either way update `docs/explain/`, the goldens and the design doc.

## Comments

2026-10-28 — The tool keeps its ids. F04 is parameter count, F05 is nesting, F06 is physical length, and F11 is logical lines. Commented-out code is not a Phase 1 rule.

2026-10-28 — Commented-out code is out of scope for this effort. It returns only if the destination is redrawn.

2026-10-28 — The correction is an ADR. The explain pages and the goldens already use the shipped ids, so they stay. The external design spec stays the historical source.

## Answer

Resolved 2026-10-28 with Christopher. The shipped rule ids stand. F04 is parameter count, F05 is nesting, F06 is physical length, and F11 is logical lines. Commented-out code is out of scope and gets no id. The explain pages and the goldens are unchanged. [ADR 0029](../../../docs/adr/0029-shipped-rule-ids-stand.md).
