# Whether a member chain saved in locals still counts

Type: grilling
Status: resolved
Blocked by:

## Question

The glossary defines a member chain as consecutive `.` or `->` hops in one expression, [ADR 0009](../../../docs/adr/0009-passthrough-warns-chain-gates.md) counts depth that way, and the adapter scores one expression, so assigning each hop to a local clears F08. `docs/explain/F08.txt` says the opposite: its anti-fix is "Saving the chain in locals leaves the finding in place," and its After example is that split. Decide which stands. If the split still counts, say how far a name is joined inside one function without symbol resolution. If one expression stands, the explain page is the thing that changes.

## Answer

Resolved 2026-10-30 with Christopher. A member chain is one expression, as the glossary and [ADR 0009](../../../docs/adr/0009-passthrough-warns-chain-gates.md) already said. Saving each hop in its own local clears the finding. Assigning the whole chain to one local leaves the hops in place, and that sentence is the finding's anti-fix and the first line of `docs/explain/F08.txt`. The After example stays the per-hop split. The other two anti-fixes stay.

## Comments

2026-10-30 — One expression stands. Saving each hop in its own local clears the finding. An alias stays unfollowed, the same as a pass-through. The explain page and the finding's anti-fix still say the opposite.
