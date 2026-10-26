# Gate on the max finding or on slop_score

Type: grilling
Status: open
Blocked by: 

## Question

The spec's open question: does the run fail on any single Tier A finding over its gate threshold (crap4java style), on the aggregate `slop_score` crossing a configured ceiling, or on both? Max is easier to explain to a model; aggregate is harder to game but harder to act on. The spec leans: gate on max for Tier A rules, report the aggregate. Decide, and decide whether `slop_score` ceilings are configurable in Phase 1 at all.
