# Validation corpus per language

Type: grilling
Status: open
Blocked by: 08

## Question

Which clean open-source repos form the validation corpus for each of Python, Rust and C++ (how many — the spec said five for TS; three per language may be enough), chosen for what (size range, test suites that emit coverage, reputation for design quality)? Your own Python projects (auto_midi, bow_arc_extraction, roomverb, solo_string_livener) are candidates for the Python side. How is the deliberately sloppy agent-written repo produced per language — prompt an agent with no quality constraints against a seeded task, or hand-write fixtures that violate each rule? What is the pass criterion: clean repos produce zero gate findings, sloppy repos trip every rule they should?
