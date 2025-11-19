# Ultra-Realistic Fire Simulation Demo Usage

## Quick Start

Run the ultra-realistic fire simulation demo:

```bash
cargo run --bin ultra-demo
```

## Command Line Options

The demo supports full customization via command-line arguments:

```bash
cargo run --bin ultra-demo -- [OPTIONS]
```

### Options

- `--size <SIZE>` - Fire size (default: medium)
  - `small` - 16 fuel elements (4×4 grid)
  - `medium` - 100 fuel elements (10×10 grid)
  - `large` - 225 fuel elements (15×15 grid)
  - `huge` - 625 fuel elements (25×25 grid)

- `--duration <SECONDS>` - Simulation duration (default: 60)

- `--terrain <TYPE>` - Terrain type (default: hill)
  - `flat` - Flat terrain at 0m elevation
  - `hill` - Single 80m hill in center
  - `valley` - Valley between two hills

- `--suppression / --no-suppression` - Enable/disable water drops (default: true)

## Examples

### Small Fire on Hill (Quick Test)
```bash
cargo run --bin ultra-demo -- --size small --duration 30
```

### Large Fire on Flat Terrain
```bash
cargo run --bin ultra-demo -- --size large --terrain flat --duration 60
```

### Huge Fire Without Suppression
```bash
cargo run --bin ultra-demo -- --size huge --duration 90 --no-suppression
```

### Valley Fire with Suppression
```bash
cargo run --bin ultra-demo -- --size medium --terrain valley --suppression
```

## Expected Results

### Small Fire (16 elements)
- Burns 2-4 elements initially
- Reaches 500-800°C
- Total fuel: ~50 kg
- Active cells: ~1000
- Duration: Burns out naturally in 20-30s

### Medium Fire (100 elements)
- Burns 5-8 elements initially
- Reaches 1200-1400°C
- Total fuel: ~500 kg
- Active cells: ~3000
- Duration: Sustained for 60+ seconds

### Large Fire (225 elements)
- Burns 10-15 elements initially
- Reaches 1400-1500°C
- Total fuel: ~1800 kg
- Active cells: ~5000
- Duration: Sustained for 90+ seconds

### Huge Fire (625 elements)
- Burns 20-30 elements at peak
- Reaches 1500°C+
- Total fuel: ~6250 kg
- Active cells: ~8000
- Duration: Sustained for 120+ seconds

## Output Interpretation

The demo prints a real-time table:

```
Time | Burning | Active Cells | Max Temp | Fuel Consumed
-----|---------|--------------|----------|---------------
   0s |       5 |         2691 |      71°C |    0.08 kg
   2s |       5 |         2691 |     172°C |    0.24 kg
   ...
```

- **Burning** - Number of elements currently on fire
- **Active Cells** - Grid cells being actively simulated
- **Max Temp** - Highest temperature in any cell (°C)
- **Fuel Consumed** - Total fuel burned so far (kg)

## Physics Features Demonstrated

1. **Realistic Fire Spread** - Fire spreads to adjacent elements via atmospheric heating
2. **Temperature Dynamics** - Reaches realistic fire temperatures (800-1500°C)
3. **Grid Coupling** - Fuel elements heat atmospheric cells
4. **Buoyancy** - Hot air rises, creating updrafts
5. **Oxygen Depletion** - Burning consumes oxygen
6. **Combustion Products** - CO₂, water vapor, and smoke generation
7. **Suppression Effects** - Water drops cool fire and increase humidity
8. **Terrain Influence** - Fire behavior affected by elevation and slope

## Performance Notes

- **Small**: Very fast, <1s per timestep
- **Medium**: Fast, ~1-2s per timestep
- **Large**: Moderate, ~3-5s per timestep
- **Huge**: Slower, ~8-12s per timestep

The simulation uses adaptive grid activation, so performance scales with active fire area, not total fuel count.

## Troubleshooting

**Fire dies too quickly?**
- Increase `--duration`
- Use `--no-suppression`
- Try larger `--size`

**Simulation too slow?**
- Use smaller `--size`
- Reduce `--duration`
- Use `--terrain flat` (fewer elevation calculations)

**Want more spread?**
- Use `hill` or `valley` terrain (slope enhances spread)
- Larger fire sizes spread more
- Disable suppression

## Legacy Demo

The original demo is still available:

```bash
cargo run --bin demo-headless
```

This uses the legacy FireSimulation system without the ultra-realistic atmospheric grid.
