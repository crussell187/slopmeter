# Rule Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rules::evaluate` turns one language's measurements into one finding per rule per function, at the worst site, using the built-in profile and the waiver lines recorded on the function.

**Architecture:** `analyzers::measure` copies the facts the engine must not recompute: logical lines, the deepest decision's line, the waiver lines, and an absent CRAP. `rules::evaluate` reads those measurements and one `LanguageProfile`. It does not read source, and it does not decide which functions were touched. A held waiver stays a gate finding the verdict can ignore. Weights stay 0 until the validation run sets them.

**Tech Stack:** Rust stable, edition 2021. No new dependencies.

## Global Constraints

- A value greater than the gate fails. The gate number itself passes. F03 warns above 10 and gates above 15. F07 warns at 80 percent or more and has no gate. F08 gates above 2 hops from the receiver and above 1 otherwise.
- The stand-ins live on the built-in profile: F01 gate 8, F02 gate 8, F04 gate 3, F05 gate 2, F06 gate 19, F11 gate 19. F02 emits nothing while CRAP is absent.
- One finding per rule per function. F01, F02, F03, F04, F06, and F11 are sited at the function line. F05 is sited at the deepest decision, the earlier line when two share that depth. F07 is sited at the forwarding call. F08 is the chain that crossed further; an equal fraction keeps the earlier line.
- Function length is the inclusive body span. The forward ratio is an integer percent, and no ordinary parameters means no F07 finding.
- A waiver holds only while the finding is a gate and the measured value is at most the written number. The threshold on the finding stays the gate. F02's number may be fractional. Every other gate takes a whole number. A missing rule, a rule that is not a gate, a missing number, a missing reason, a fraction on a whole-number gate, and the same rule twice are rejected.
- A rejected line on a metric that is already over its gate stays on that one finding. Any other rejected line is its own gate finding, weight 0, sited at the waiver line. A valid waiver on a measurement that is not over its gate emits nothing.
- The finding's principle is the explain page's opening sentence. Its anti-fix is the first anti-fix line. Its fix shape names the refactoring and fills in the measured value, the threshold, and the function.

---

### Task 1: Carry the facts the engine reads

**Files:**
- Modify: `src/analyzers.rs`
- Modify: `src/inputs/config.rs`
- Test: `src/analyzers.rs`, `src/inputs/config.rs`

**Interfaces:**
- Consumes: `ir::Function` (`body.lloc`, `body.decisions`, `waivers`)
- Produces: `Measurement` gains `lloc`, `nesting_line`, `waivers`, and `crap: Option<f64>`. `measure` sets `crap` to `None`. `LanguageProfile` gains `f01_gate`, `f02_gate`, `f04_gate`, `f05_gate`, `f06_gate`, and `f11_gate`.

- [x] **Step 1: Write the failing test**

A body with logical lines, two decisions at the deepest depth, and one waiver line. The measurement keeps the logical lines, the earlier deepest line, and the waiver. CRAP is absent.

- [x] **Step 2: Run it and watch it fail**

Run: `cargo test --lib analyzers::tests::the_measurement_keeps_lines_nesting_and_waivers`
Expected: FAIL to compile because those fields do not exist.

- [x] **Step 3: Copy the facts and publish the stand-in gates**

- [x] **Step 4: Run the analyzer and config tests**

Run: `cargo test --lib analyzers:: inputs::config::`
Expected: PASS

### Task 2: Emit one finding per metric

**Files:**
- Create: `src/rules.rs`
- Modify: `src/lib.rs`
- Test: `src/rules.rs`

**Interfaces:**
- Consumes: `analyzers::Measurement`, `inputs::config::LanguageProfile`
- Produces:

```rust
pub fn evaluate(measurements: &[Measurement], profile: &LanguageProfile) -> Vec<Finding>
```

`Finding` carries `rule`, `level` (`Gate` or `Warn`), `path`, `name`, `param_types`, `qualifier`, `line`, `measured_value`, `threshold`, `weight`, `principle`, `fix_shape`, `anti_fix`, and `waiver`. `weight` is 0. `waiver` is `None` until Task 3.

- [x] **Step 1: Write the failing tests**

Cyclomatic complexity 9 emits F01 at the function line, with the McCabe principle, the first anti-fix line, and a fix shape that names the function, 9, and gate 8. Complexity 8 emits nothing. The same boundary, one past it, holds for F02 through F11, including F03's warn band and F08's worse chain. A raised `f01_gate` on the profile suppresses F01.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib rules::`
Expected: FAIL to compile because `rules` does not exist.

- [x] **Step 3: Implement the nine metrics**

- [x] **Step 4: Run the library tests**

Run: `cargo test --lib`
Expected: PASS

### Task 3: Honor and reject waiver lines

**Files:**
- Modify: `src/rules.rs`
- Test: `src/rules.rs`

**Interfaces:**
- Produces: `Waiver { excused, reason, holds, rejection }` on the finding when a line for that rule is attached. `holds` is true only for a gate whose measurement is at most the excused number.

- [x] **Step 1: Write the failing tests**

A whole number at or above the measurement holds F01 and does not also emit a warn for an F03 gate. A number below the measurement stays a gate with `holds` false. A missing reason on an over-gate metric stays on that finding. A missing reason with no metric gate is a gate finding at the waiver line. F07, a fraction on F01, a missing rule, and the same rule twice are rejected. A valid F02 fraction holds. A valid waiver under the gate emits nothing. A rejected F03 line in the warn band replaces the warn with one gate finding.

- [x] **Step 2: Run them and watch them fail**

Run: `cargo test --lib rules::tests::waiver`
Expected: FAIL on `holds` or on a missing rejection finding.

- [x] **Step 3: Classify each recorded line and attach it**

- [x] **Step 4: Run fmt, clippy, and the full suite**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: fmt prints nothing, clippy prints nothing, tests PASS.

- [x] **Step 5: Commit**

```bash
git add src/analyzers.rs src/inputs/config.rs src/rules.rs src/lib.rs
git commit -m "Turn measurements into findings."
```
