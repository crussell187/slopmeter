---
status: accepted
date: 2026-10-26
---

# Phase 1 IR is files and functions, in memory

The spec's IR mixes module graphs, class matrices, call edges, churn, and coverage into the document the function rules read, and it stores judgments such as `is_passthrough` beside the measurements. Phase 1 stores the scored files and their functions. A file has its repo-relative path, one language tag, its unparsed ratio, and its functions. A function has its identity, its body span, its touch span, its parameters, and the adapter's classification of the body: a logical-line count, the decision lines with their depth, the cognitive increments, the member-chain hops, and the forwarding call when the body is one call. Analyzers derive the measurements from that classification. The rule engine applies thresholds. Coverage stays in the LCOV report and joins on the body span.

The value lives in memory for the run. It is not written to disk, and it has no version field. The Rust types are the schema. One binary produces the value and consumes it.

## Considered options

- **The spec's document, with classes, edges, churn, coverage, and `is_passthrough`.** Rejected. Classes, imports, call edges, and churn are later phases. Coverage is a report joined by span. `is_passthrough` is the 80% threshold, which the rule engine owns.
- **A language-neutral statement tree.** Rejected. Every language construct would leak into the schema. The adapter classifies; later stages do not see source text.
- **A versioned file cache.** Rejected. Nothing in Phase 1 reads a previous IR. The JSON report is the artifact that leaves the process.

## Consequences

- Identity and the doc-comment rule for the touch span stay in [ADR 0002](0002-what-counts-as-a-function.md) and [ADR 0008](0008-what-changed-means.md).
- Each function carries its cognitive increments as line, kind, and depth. The sum is [Cognitive complexity follows the white paper](0020-cognitive-complexity-increments.md).
- The logical-line count is the number of statements in the body. [A logical line is a statement](0021-logical-line-is-a-statement.md). The field is an integer the adapter fills.
- The sample that tried the shape on three real files is `prototype/ir-schema/`. Its `version` field was dropped.
- Each function also carries the waiver lines the adapter recorded above its touch span. The rule engine reads that record. [A waiver excuses one gate up to a stated measurement](0023-a-waiver-excuses-one-gate.md).
