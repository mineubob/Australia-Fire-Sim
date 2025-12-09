# Agent Task: Migrate f32 → More Appropriate Numeric Types

```
Migrate f32 safely: pilot internal f32→f64 conversions in critical physics (crates/core/src/physics/element_heat_transfer.rs, rothermel.rs, grid/wind_field.rs), keep public/FFI types unchanged unless authorized. Use f64::from for lossless casts, annotate downcasts, add tests & micro-benchmarks, run workspace tests + clippy, measure perf, create branch+PR with rollback plan.
```

## Summary
Migrate f32 usages in the repository where higher precision (f64), integer types, or other types are more appropriate. The agent will use a safe, staged approach: pilot high-value hotspots, validate tests/benchmarks, then progressively widen conversion. Public/FFI APIs are not changed unless explicitly requested.

This task is designed to be executed by an automated coding agent which will create a branch and open a pull request when launched (see "How the agent runs / PR behavior" below).

---

## Scope & Priorities (Pilot → Rollout)
1. Pilot (do first):
   - `crates/core/src/physics/element_heat_transfer.rs` (Stefan–Boltzmann T^4, pow/exp hotspots)
   - `crates/core/src/physics/rothermel.rs` (powf/exp heavy math)
   - `crates/core/src/grid/wind_field.rs` (solver math)

2. Next wave:
   - `crates/core/src/core_types/ember.rs` (heat/cooling math)
   - Other physics files using powf/powi/exp extensively

3. Broader conversions (only after pilot validated):
   - Consider simulation loop internals, numeric accumulators, or public APIs.
   - FFI and demo CLI conversions only with explicit permission and a separate breaking-change PR.

Files / paths excluded by default:
- UI/demo visual-only code unless requested
- Files flagged explicitly by maintainers

---

## Rules / Standards / Safety
- Use f64 for intermediate computations that involve powf/powi/exp or T^4, then downcast to f32 at stable API boundaries if the API must remain f32.
- Use `f64::from(x)` for lossless conversions from f32 to f64.
- Keep the public/FFI signatures unchanged by default. If a breaking change path is approved, do that in a separate PR.
- Annotate any deliberate downcasts with either `#[expect(clippy::cast_precision_loss)]` or `#[allow(clippy::cast_precision_loss)]` with a comment explaining the decision.
- Add tests asserting expected numerical behavior (where possible) and add micro-benchmarks for hotspots to measure performance impact.
- Changes should be made in small, reviewable commits (pilot -> followups).

---

## Implementation Plan (Agent steps)
1. Baseline
   - Run `cargo test --workspace` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
   - Run existing benchmarks or a custom micro-benchmark for hot functions (if available) and save results.

2. Pilot conversion (strict small commits)
   - Add targeted unit tests where current outcomes matter and would detect regressions.
   - Convert computations to f64 internally in the pilot files (constants → f64, intermediate math → f64).
   - Use `f64::from(...)` for conversions and annotate downcasts as necessary.
   - Re-run tests and clippy; fix any new lints.
   - Run benchmarks again and capture perf diff.

3. Expand conversions in small batches, repeating the test & perf cycle.

4. Breaking-change PR (optional)
   - If the user authorizes public API changes, do these in a dedicated PR with full migration (FFI, demo, callsites).
   - Provide a migration guide and compatibility layer if feasible.

5. PR & CI
   - Create a branch like `migrate/f32-to-better-types/<scope>`.
   - Commit pilot and follow-up changes with tests and benchmark artifacts or summary.
   - Open PR with description: scope, tests, benchmarks, perf diff, risk/rollback plan.

---

## Tests & Benchmarks
- Run `cargo test --workspace` for correctness
- Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` for lints
- Add (or run) `cargo bench` if available; otherwise add micro-benchmarks in `crates/core/benches` targeting hotspots
- Performance acceptance threshold: default <5% slowdown in hot loops — otherwise propose optimizations or revert

---

## PR & Rollback behavior
- PR will be created automatically by the coding agent if instructed to `create_pr`.
- The PR should include CI results and a perf summary. If the agent detects failing tests or unacceptable regression, it will stop and propose a revert/adjust plan.

---

## Reviewer checklist for the PR
- All CI unit & integration tests pass
- Clippy/format/lint checks pass
- Benchmarks included and within acceptable perf thresholds
- No unannotated or unexplained precision-limiting casts
- No accidental changes to public API unless explicitly requested

---

## Meta: How this runs (will it auto-create a PR?)
Yes — when you *launch* the agent task with the repository-integrated coding agent (for example, via the copilot coding agent or an automated GitHub action the coding agent triggers), the automated agent will:
- Create a new branch
- Apply changes in small commits
- Run tests and clippy
- Open a pull request against `main` with a detailed description, test results, and benchmarks
If you prefer not to auto-open a PR, instruct the agent to only produce a branch or local changes for manual review.

---

## Launch options (choose at invocation)
- `pilot-only` (default): only pilots the high-value files without changing public APIs
- `aggressive` (breaking): convert public APIs and FFI types; will update all call sites and demo — requires explicit confirmation
- `dry-run`: produce a patch/branch locally and run tests but do not open a PR

---

## Example PR title & description templates the agent will use
Title: "pilot(physics): migrate element_heat_transfer internals to f64 (no API change)"
Description: - What changed - Files/commits - Tests & CLA/CI results - Benchmarks & perf results - Follow-ups

---

If you say "Go", the agent will create a branch, run the pilot conversion(s), run tests & clippy, collect benchmark results, and open a PR. If you just want the task file created (this file), no PR will be created until you instruct the agent to run.


---

<!-- End-of-task file -->
