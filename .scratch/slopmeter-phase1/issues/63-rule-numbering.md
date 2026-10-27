# Reconcile rule ids with the spec and the dropped commented-out-code rule

Type: grilling
Status: open
Blocked by: 

## Question

The spec numbered F04 logical lines, F05 parameters, F06 nesting and F11 commented-out code; the build ships F04 parameters, F05 nesting, F06 physical length and F11 logical lines, and no rule, explain page or ADR covers commented-out code (review, spec drift). The design doc, the explain pages and any agent prompt written from the spec now disagree with the tool. Decide: keep the build's ids and record the mapping and the dropped rule in an ADR, or renumber to the spec and implement commented-out code (a comment block that parses as code in the host language) as F11. Either way update `docs/explain/`, the goldens and the design doc.
