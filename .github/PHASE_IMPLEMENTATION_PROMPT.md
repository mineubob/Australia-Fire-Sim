# Phase Implementation Prompt Template

Use these prompts to have Copilot implement fire physics phases using sub-agents.

**Environment:** Local development with GPU available

---

## Multi-Phase Implementation (Recommended)

```
Implement Phases [X-Y] from .github/agent-tasks/FIRE_PHYSICS_ENHANCEMENTS.md.

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow physics formulas EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core [phase_keyword]
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

---

## Single Phase Implementation

```
Implement Phase [X] from .github/agent-tasks/FIRE_PHYSICS_ENHANCEMENTS.md using a sub-agent.

Requirements:
1. Read the full phase specification from the task file first
2. Implement all deliverables listed in the phase
3. Follow the physics formulas EXACTLY as documented
4. NEVER simplify or approximate the science
5. All public APIs must have documentation comments
6. Create unit tests for each new function/struct

Validation (must pass before reporting done):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core [phase_keyword]
- cargo fmt --all --check

Do NOT stop until the phase is fully implemented and all validation passes.
Report back with: files created, tests passing, and any issues encountered.
```

---

## Task File Reference

| Phases | Task File |
|--------|-----------|
| 0-4 (Core) | `.github/agent-tasks/FIRE_PHYSICS_ENHANCEMENTS.md` |
| 5-8 (Advanced) | `.github/agent-tasks/FIRE_PHYSICS_ADVANCED.md` |

---

## Phase Quick Reference

### Core Phases (FIRE_PHYSICS_ENHANCEMENTS.md)

| Phase | Name | Test Keywords |
|-------|------|---------------|
| 0 | Terrain Slope Integration | `terrain slope uphill` |
| 1 | Vertical Fuel Layers | `fuel_layer vertical_heat` |
| 2 | Fuel Heterogeneity | `noise fuel_variation` |
| 3 | Crown Fire Transition | `crown_fire` |
| 4 | Pyroconvection Dynamics | `atmosphere convection downdraft` |

### Advanced Phases (FIRE_PHYSICS_ADVANCED.md)

| Phase | Name | Test Keywords |
|-------|------|---------------|
| 5 | Junction Zone Physics | `junction_zone` |
| 6 | VLS (Vorticity Lateral Spread) | `vls vorticity` |
| 7 | Valley Channeling | `valley chimney` |
| 8 | Plume/Wind Regime Detection | `regime byram` |

---

## Example Prompts

### Implement Core Phases 0-2
```
Implement Phases 0-2 from .github/agent-tasks/FIRE_PHYSICS_ENHANCEMENTS.md.

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow physics formulas EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core [phase_keyword]
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

### Implement Core Phases 3-4
```
Implement Phases 3-4 from .github/agent-tasks/FIRE_PHYSICS_ENHANCEMENTS.md.

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow physics formulas EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core [phase_keyword]
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

### Implement Advanced Phases 5-8
```
Implement Phases 5-8 from .github/agent-tasks/FIRE_PHYSICS_ADVANCED.md.

For EACH phase sequentially, use a sub-agent with these instructions:

1. Read the full phase specification from the task file
2. Implement ALL deliverables listed for this phase
3. Follow physics formulas EXACTLY as documented - NEVER simplify
4. All public APIs must have documentation comments
5. Create unit tests for each new function/struct

Validation (must pass before moving to next phase):
- cargo build -p fire-sim-core
- cargo clippy --all-targets --all-features (ZERO warnings)
- cargo test -p fire-sim-core [phase_keyword]
- cargo fmt --all --check

After ALL phases complete, summarize:
1. Files created per phase
2. Tests passing per phase
3. Any issues that need manual intervention
```

---

## Notes

| Consideration | Detail |
|---------------|--------|
| **Sequential execution** | Sub-agents run one after another — later phases see earlier code |
| **No branching needed** | Each phase commits directly, linear history |
| **GPU available** | Uses `--all-features` for full GPU shader validation |
| **Physics accuracy** | NEVER allow simplification — formulas must match published literature |
