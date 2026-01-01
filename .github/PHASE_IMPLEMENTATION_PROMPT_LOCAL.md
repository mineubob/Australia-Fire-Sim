# Phase Implementation Prompt Template for Local Development

Use these prompts to have Copilot implement multi-phase tasks using sub-agents.

**Environment:** Local development with GPU available

**Usage:** Replace `[TASK_FILE]` and `[X-Y]` with actual values from your task file.

---

## Multi-Phase Implementation (Recommended)

```
Implement Phases [X-Y] from [TASK_FILE].

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow all specifications EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core (use keywords from task file for specific phase tests)
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

---

## Single Phase Implementation

```
Implement Phase [X] from [TASK_FILE] using a sub-agent.

Requirements:
1. Read the full phase specification from the task file first
2. Implement all deliverables listed in the phase
3. Follow all specifications EXACTLY as documented
4. NEVER simplify or approximate requirements
5. All public APIs must have documentation comments
6. Create unit tests for each new function/struct

Validation (must pass before reporting done):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core (use keywords from task file for specific phase tests)
- cargo fmt --all --check

Do NOT stop until the phase is fully implemented and all validation passes.
Report back with: files created, tests passing, and any issues encountered.
```

---

## Using These Prompts

1. **Identify your task file** - Locate the markdown file containing phase specifications
2. **Choose appropriate prompt** - Single phase or multi-phase implementation
3. **Replace placeholders** - Update `[TASK_FILE]` and `[X-Y]` with actual values
4. **Run the prompt** - Copilot will spawn sub-agents to implement each phase sequentially

**Note:** You don't need to list phases in the prompt - sub-agents will read them directly from the task file.

---

## Example Prompts

### Multiple Phases
```
Implement Phases [X-Y] from [TASK_FILE].

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow all specifications EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core (use keywords from task file for specific phase tests)
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

### Single Phase
```
Implement Phase [X] from [TASK_FILE] using a sub-agent.

Requirements:
1. Read the full phase specification from the task file first
2. Implement all deliverables listed in the phase
3. Follow all specifications EXACTLY as documented
4. NEVER simplify or approximate requirements
5. All public APIs must have documentation comments
6. Create unit tests for each new function/struct

Validation (must pass before reporting done):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core (use keywords from task file for specific phase tests)
- cargo fmt --all --check

Do NOT stop until the phase is fully implemented and all validation passes.
Report back with: files created, tests passing, and any issues encountered.
```

---

## Notes

| Consideration | Detail |
|---------------|--------|
| **Sequential execution** | Sub-agents run one after another — later phases see earlier code |
| **No branching needed** | Each phase commits directly, linear history |
| **GPU available** | Uses `--all-features` for full GPU shader validation |
| **Accuracy** | NEVER allow simplification — implement exactly as documented in task file |
| **Sub-agent isolation** | Each sub-agent completes one phase independently with full validation |
