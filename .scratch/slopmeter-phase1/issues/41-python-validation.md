# Run the Python corpus

Type: task
Status: resolved
Blocked by: 40

## Question

Implement session 14 of [the Phase 1 plan](../../../docs/superpowers/plans/2026-10-27-phase1.md). `--strict` on the three pinned Python repos moves stand-in thresholds until no gate finding remains. The sloppy tree is pinned once every gating rule fires and F07 warns. The corpus run stays off CI.

## Answer

`slopmeter [flags] [path]` scores a tree. `--strict` puts every finding in the gating list. The pins are `corpus/python/pins.txt`: `psf/requests` `3c7baaa05ab4aa49cc819ce3a819d260de19c157`, `encode/httpx` `b5addb64f0161ff6bfe94c124ef76f6a1fba5254`, `pallets/click` `2247b35ea1c47c727d7a06e51fa280e12a863ff6`. Python gates moved to the highest value that still passes: F01 48, F03 58, F04 19, F05 6, F06 196, F08 3 hops from the receiver and 3 otherwise, F11 64. F02 stays 8. No coverage report was given, so CRAP was not computed. F03's warn stays 10 and F07 stays 80. The clean repos still warn on F03 and F07. Rust and C++ keep the stand-ins. `corpus/python/sloppy` was written from `corpus/python/seed.txt`. With its coverage report, a strict run gates on F01–F06, F08, and F11 and warns on F07. The run is not in CI. The code-level plan is [Python validation](../../../docs/superpowers/plans/2026-10-27-python-validation.md).
