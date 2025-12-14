# Australia Fire Simulation - Development Guidelines

Scientifically accurate wildfire simulation. **Extreme realism is paramount.**

**Status:** Phase 1-3 Complete — Rothermel, Van Wagner, Albini, Nelson, Rein models integrated. 83 unit tests passing.

---

## CORE RULES

### 1. NEVER SIMPLIFY PHYSICS
- Implement formulas exactly as published in fire science literature
- Stefan-Boltzmann uses full `(T_source^4 - T_target^4)` — no approximations
- Moisture evaporation (2260 kJ/kg latent heat) happens BEFORE temperature rise
- Wind creates extreme asymmetry: 26x faster downwind, 0.05x upwind
- Fire climbs 2.5x+ faster than horizontal spread

### 2. NEVER HARDCODE DYNAMIC VALUES
Values that vary by context MUST come from the appropriate struct:

| Category | Example Wrong | Example Correct |
|----------|---------------|-----------------|
| Fuel | `let cooling_rate = 0.1;` | `fuel.cooling_rate` |
| Weather | `let wind_speed = 10.0;` | `weather.wind_speed` |
| Grid | `let oxygen = 0.21;` | `cell.oxygen` |
| Time | `let change = 0.01;` | `rate * dt` |

**Exception:** Universal physical constants (e.g., `STEFAN_BOLTZMANN = 5.67e-8`, `GRAVITY = 9.81`) CAN be hardcoded.

### 3. AUSTRALIAN-SPECIFIC BEHAVIORS
- Eucalyptus oil explosions (vaporizes 170°C, autoignites 232°C, 43 MJ/kg)
- Stringybark ladder fuels dramatically lower crown fire thresholds
- Ember spotting up to 25km (validated against Black Saturday data)

### 4. PUBLIC CODE SHOULD BE COMMENTED
- Public-facing APIs (public crates, modules, types, and functions) MUST include at least minimal documentation/comments explaining purpose and expected usage.
- Keep comments concise and factual: a one-line summary for each public item plus any important invariants, side-effects, or usage notes is sufficient.

### 5. FIX ROOT CAUSES, NOT SYMPTOMS
- **NEVER apply band-aid fixes** that mask underlying problems with artificial limits, clamping, or workarounds.
- When encountering invalid values (negative temperatures, NaN, out-of-range, etc.), investigate **WHY** they occur.
- Fix the physics/math that creates the problem, not the symptom.
- Example: Temperature going below absolute zero? Don't just clamp to -273.15°C — fix the cooling calculation to use stable exponential decay instead of unstable explicit Euler integration.
- Artificial limits are acceptable ONLY for physical constraints (max temperature 800°C for finite precision, oxygen can't exceed atmospheric levels) — not to hide numerical instability or incorrect formulas.

---

## IMPLEMENTED PHYSICS MODELS

| Model | Source | Purpose |
|-------|--------|---------|
| Rothermel (1972) | USDA INT-115 | Surface fire spread rate |
| Van Wagner (1977) | Can. J. For. Res. | Crown fire initiation |
| Albini (1979, 1983) | USDA Research Papers | Ember spotting/lofting |
| Nelson (2000) | USDA Southern Station | Fuel moisture timelag |
| Rein (2009) | Int. Rev. Chem. Eng. | Smoldering combustion |
| McArthur FFDI Mk5 | BoM Australia | Fire danger rating |
| Byram (1959) | Fire intensity | Flame height: L = 0.0775 × I^0.46 |

---

## CODEBASE STRUCTURE

```
crates/core/src/
├── core_types/    # Fuel, FuelElement, Ember, Weather, SpatialIndex
├── physics/       # Rothermel, Albini, crown fire, combustion, suppression
├── grid/          # 3D atmospheric grid with terrain
└── simulation/    # Main loop integrating all systems
```

**Key files to reference:**
- `core_types/fuel.rs` — 8 fuel types with 30+ properties each
- `physics/element_heat_transfer.rs` — Stefan-Boltzmann radiation, wind/slope effects
- `core_types/weather.rs` — FFDI calculation, 6 WA regional presets

---

## RUNNING THE DEMO

The `demo-interactive` binary has two modes:

### Interactive Mode (default)

Launches a full-featured terminal UI with ratatui:
```bash
cargo run --release --bin demo-interactive
```

Provides a multi-panel interface with real-time visualization, command input, and status displays.

### Headless Mode

**AI agents MUST use `--headless` to interact with the demo.** Without this flag, the demo enters interactive TUI mode which cannot be driven programmatically.

For automation, scripting, or non-interactive use, run with the `--headless` flag:

**Building with symbols for profiling:**
```bash
CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --bin demo-interactive
```

**Running with heredoc input:**
```bash
./target/release/demo-interactive --headless <<'HEREDOC'
1000
1000
p perth
i 7
s 100
q
HEREDOC
```

Annotated: what each line in the heredoc is doing

- 1000 — terrain width (meters). Optional — press Enter or omit to use default of 150.0 meters.
- 1000 — terrain height (meters). Optional — press Enter or omit to use default of 150.0 meters.
- p perth — select a weather preset ("p" is the demo command to choose a preset, then the preset name). Optional — if omitted the demo will use its default preset.
- i 7 — ignite element id 7 ("i" is the demo command to ignite the specified fuel element ID). **Required** to start a fire.
- s 100 — step the simulation 100 times ("s" steps forwards in time; each step is one internal physics timestep). **Required** to advance the simulation.
- q — quit the demo ("q" is the demo command to quit). Optional if stdin closes naturally.

Notes:
- You can change the terrain dimensions and commands to other values, or leave a prompt blank in the heredoc to accept demo defaults.
- Use quoted heredoc (<<'HEREDOC') to prevent shell expansion and ensure the demo receives the lines exactly as shown.
- Headless mode outputs log messages to stdout, suitable for piping or redirection.

---

## AI AGENT RULES

### Completion Rule
AI agents MUST NOT stop, pause, or ask permission until the user's request is fully implemented. Continue working until done or genuinely blocked.

### Always View Full Output
Never make decisions based on incomplete information.

**NEVER TRUNCATE OUTPUT.** Do not use `head`, `tail`, `grep | head`, or any pipeline that limits output before it's saved. These commands hide critical information and lead to incorrect conclusions.

**For potentially large output, use `tee` to view and save simultaneously:**
```bash
# Use tee to see output AND save to file
command 2>&1 | tee output.txt

# If tee is unavailable, redirect to file instead
command > output.txt 2>&1

# Then read the file
cat output.txt  # or use read_file tool
```

**Important:** Delete temporary output files before committing. Do not upload them to GitHub.

**Required practices:**
- **Test output:** Use `wc -l` to count lines before viewing directly
- **Large output:** Redirect to file (`command > output.txt 2>&1`), then read file
- **Cargo commands:** Always redirect: `cargo clippy > clippy.txt 2>&1`
- **Git commands:** Use `git --no-pager` AND redirect for complete output
- **Read everything:** Never make decisions on partial data

### Validate Rust Code Before Submitting
```bash
cargo clippy --all-targets --all-features
cargo fmt --all -v --check
```
**CRITICAL:** Fix ALL warnings by changing code — NEVER use `#[allow(...)]` macros. Workspace `Cargo.toml` denies both rustc and clippy warnings (equivalent to `-D warnings`), so any warning will fail the build.

---

## COMMON PITFALLS

| ❌ Don't | ✅ Do |
|----------|-------|
| Skip moisture evaporation | Heat → evaporation FIRST, then temperature |
| Use simplified Stefan-Boltzmann | Full T^4 formula with emissivity |
| Hardcode fuel properties | Use `fuel.property` |
| Assume uniform grid conditions | Query `cell.oxygen`, `cell.temperature` |
| Suppress clippy warnings | Fix the code |
| Simplify for performance | Profile first, then optimize |
| Simplify scientific formulas | Implement exactly as published in literature |

---

## KEY TAKEAWAYS

1. This is a **simulation, not a game** — extreme realism is the goal
2. **Never simplify formulas** — implement exactly as published
3. **Australian fire behavior is unique** — eucalyptus oils, stringybark, 25km spotting
4. **Moisture evaporation is critical** — 2260 kJ/kg latent heat FIRST
5. **Wind effects are extreme** — 26x downwind is realistic
6. **Validation is mandatory** — tests against known values
7. **User's mantra:** "I want it as realistic as possible — NEVER simplify"

---

## REFERENCES

- Rothermel (1972) — USDA Forest Service Research Paper INT-115
- Van Wagner (1977) — Canadian Journal of Forest Research
- Albini (1979, 1983) — USDA Forest Service Research Papers
- Nelson (2000) — Forest Service Southern Research Station
- Rein (2009) — International Review of Chemical Engineering
- McArthur FFDI Mk5 — Bureau of Meteorology, Australia
- WA Fire Behaviour Calculator — https://aurora.landgate.wa.gov.au/fbc/
