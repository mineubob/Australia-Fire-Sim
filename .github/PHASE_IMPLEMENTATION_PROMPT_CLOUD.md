# Phase Implementation Prompt Template for Cloud Agents

Use these prompts with the GitHub Copilot coding agent (cloud) to implement multi-phase tasks.

**Note:** Cloud agents do NOT support sub-agents. Use these direct implementation prompts instead.

**Environment:** Cloud agent (CPU-only, no GPU available)

**Usage:** Replace `[TASK_FILE]`, `[X-Y]`, `[phase_keyword]` with actual values from your task file.

---

## Multi-Phase Implementation (Recommended)

```
Implement Phases [X-Y] from [TASK_FILE].

IMPLEMENTATION REQUIREMENTS:

1. Read the full specification for each phase from the task file
2. Implement phases SEQUENTIALLY (complete one before starting next)
3. For EACH phase, implement ALL deliverables listed
4. Follow physics formulas EXACTLY as documented - NEVER simplify
5. All public APIs MUST have documentation comments
6. Create unit tests for each new function/struct
7. Run validation after EACH phase before proceeding to next

VALIDATION (must pass after each phase):
- cargo build -p fire-sim-core --no-default-features
- cargo clippy --all-targets --no-default-features (ZERO warnings allowed)
- cargo test -p fire-sim-core --no-default-features [phase_keyword]
- cargo fmt --all --check

IMPLEMENTATION SEQUENCE:
For each phase [X] through [Y]:
  a) Read phase specification completely
  b) Implement all deliverables
  c) Add documentation comments to all public items
  d) Create unit tests
  e) Run all validation commands
  f) Fix any errors/warnings
  g) Only proceed to next phase when ALL validation passes

FINAL DELIVERABLES:
- All phases [X-Y] fully implemented
- All tests passing
- Zero clippy warnings
- Code formatted correctly
- Summary comment listing:
  * Files created per phase
  * Tests added per phase
  * Any issues requiring manual review
```

---

## Single Phase Implementation

```
Implement Phase [X] from [TASK_FILE].

REQUIREMENTS:

1. Read the full phase specification from the task file first
2. Implement ALL deliverables listed in the phase
3. Follow all specifications EXACTLY as documented in the phase spec
4. NEVER simplify or approximate requirements
5. All public APIs (pub fn, pub struct, pub mod) MUST have documentation comments
6. Create unit tests for each new function/struct
7. Tests must validate against known values from literature

IMPLEMENTATION STEPS:

1. Read specification:
   - Open [TASK_FILE]
   - Find Phase [X] section
   - Read all specifications and requirements

2. Create implementations:
   - Add new files/modules as specified
   - Implement structs and functions
   - Add doc comments (/// or //!) to ALL public items
   - Use exact formulas from spec (no approximations)

3. Create tests:
   - Add test module or file
   - Test each function with known values
   - Validate against literature where applicable
   - Use clear test names describing what's validated

4. Validate implementation:
   - Run: cargo build -p fire-sim-core --no-default-features
   - Run: cargo clippy --all-targets --no-default-features
   - Fix ALL warnings (NEVER use #[allow(...)])
   - Run: cargo test -p fire-sim-core --no-default-features [phase_keyword]
   - Run: cargo fmt --all --check
   - Ensure ALL commands succeed

5. Final check:
   - Verify all deliverables from phase spec are complete
   - Confirm zero clippy warnings
   - Confirm all tests pass
   - Confirm code is formatted

DELIVERABLES:
- Phase [X] fully implemented
- All tests passing
- Zero clippy warnings
- Code formatted
- Summary comment with files created and tests added

DO NOT STOP until all validation passes and phase is complete.
```

---

## Complete Task Implementation (All Phases)

```
Implement ALL phases from [TASK_FILE].

REQUIREMENTS:

1. Read [TASK_FILE] to identify all phases and their specifications
2. Implement phases SEQUENTIALLY (in numerical order)
3. For EACH phase:
   - Read complete specification from task file
   - Implement ALL deliverables exactly as specified
   - Follow all specifications EXACTLY - NEVER simplify
   - Add documentation comments to ALL public APIs
   - Create comprehensive unit tests
   - Validate before proceeding to next phase

3. Accuracy is PARAMOUNT:
   - Use exact specifications from task file
   - No approximations or simplifications
   - Preserve all specified constants and parameters
   - Reference sources in doc comments where applicable

4. Code quality requirements:
   - All public items must have doc comments
   - Tests must validate against specified values
   - Zero clippy warnings allowed
   - Code must be formatted per rustfmt

VALIDATION SEQUENCE (after EACH phase):
```bash
# Must pass before proceeding to next phase
cargo build -p fire-sim-core --no-default-features
cargo clippy --all-targets --no-default-features  # ZERO warnings
cargo test -p fire-sim-core --no-default-features [phase_keyword]
cargo fmt --all --check
```

NOTE: Test keywords for each phase should be found in the task file specifications.

FINAL DELIVERABLES:
- All phases fully implemented
- All specifications implemented exactly as documented
- All tests passing (minimum 5 tests per phase recommended)
- Zero clippy warnings across entire codebase
- Code formatted correctly
- Summary listing:
  * Files created per phase
  * Number of tests per phase
  * Any issues needing manual review

CRITICAL: DO NOT simplify specifications. DO NOT skip validation. DO NOT stop until all phases complete successfully.
```

---

## Using These Prompts

1. **Identify your task file** - Locate the markdown file containing phase specifications
2. **Choose appropriate prompt** - Single phase, multi-phase, or complete implementation
3. **Replace placeholders** - Update `[TASK_FILE]`, `[X-Y]`, and `[phase_keyword]` with actual values
4. **Submit to cloud agent** - Use the hashtag to invoke the coding agent with your customized prompt

---

## Example Cloud Agent Prompts

### Quick Test - Single Phase
```
Implement Phase [X] ([Phase Name]) from [TASK_FILE].

Read the phase specification, implement all deliverables exactly as documented (NEVER simplify), add documentation comments to all public items, create unit tests, and validate with:
- cargo build -p fire-sim-core --no-default-features
- cargo clippy --all-targets --no-default-features (must have ZERO warnings)
- cargo test -p fire-sim-core --no-default-features [phase_keyword]
- cargo fmt --all --check

Do not stop until all validation passes. Report files created and tests added.
```

### Production - Multiple Phases
```
Implement Phases [X-Y] from [TASK_FILE].

Implement phases sequentially. For each phase: read spec, implement all deliverables, follow specifications EXACTLY (never simplify), add doc comments to all public APIs, create unit tests, and validate with cargo build/clippy/test/fmt (all with --no-default-features to skip GPU). 

Proceed to next phase only after all validation passes. Final deliverables: all phases complete, all tests passing, zero clippy warnings, formatted code, and summary of files/tests per phase.
```

### Production - Complete Implementation
```
Implement all phases from [TASK_FILE].

Implement phases sequentially. For each phase: read spec, implement all deliverables, follow specifications EXACTLY (never simplify), add doc comments to all public APIs, create unit tests, and validate with cargo build/clippy/test/fmt (all with --no-default-features to skip GPU).

Proceed to next phase only after all validation passes. Final deliverables: all phases complete, all tests passing, zero clippy warnings, formatted code, and summary of files/tests per phase.
```

---

## Cloud Agent Limitations & Workarounds

| Limitation | Impact | Workaround |
|------------|--------|------------|
| No sub-agents | Cannot delegate phases to separate agents | Use sequential implementation in single agent |
| No GPU support | Cannot validate GPU shaders/features | Use `--no-default-features` flag (default features include `gpu`) |
| No interactive feedback | Cannot ask clarifying questions mid-task | Provide complete requirements upfront |
| Fixed execution scope | Cannot dynamically adjust based on findings | Include comprehensive validation in prompt |
| No iterative refinement | All work done in one session | Specify all quality gates in initial prompt |

---

## Validation Command Reference

```bash
# Build check (must succeed)
cargo build -p fire-sim-core --no-default-features

# Lint check (must have ZERO warnings)
cargo clippy --all-targets --no-default-features

# Test check (must have all tests passing)
cargo test -p fire-sim-core --no-default-features              # All tests
cargo test -p fire-sim-core --no-default-features [keyword]    # Specific phase
# Use test keywords from your phase specifications

# Format check (must succeed)
cargo fmt --all --check
```

---

## Critical Reminders for Cloud Agents

1. **Never simplify specifications** - Implement exactly as documented in task file
2. **Zero warnings policy** - Workspace denies warnings, so ALL must be fixed
3. **Documentation required** - All public items need doc comments
4. **Test thoroughly** - Minimum 5 tests per phase recommended, validating against specified values
5. **Sequential execution** - Complete one phase fully before starting next
6. **Validation gates** - Must pass all checks before proceeding
7. **No shortcuts** - Implement all deliverables, no omissions

---

## Notes

- Cloud agents create pull requests automatically when done
- All work is committed to a feature branch
- Review the PR after agent completes to verify all requirements met
- Accuracy and correctness are more important than speed - take time to implement properly
- Reference project-specific guidelines (e.g., .github/copilot-instructions.md) for additional rules
