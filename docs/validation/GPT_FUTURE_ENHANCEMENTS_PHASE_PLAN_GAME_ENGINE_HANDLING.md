# Australia Fire Simulation: Multi-Phase Enhancement Plan

This plan outlines the phased implementation of missing behaviors and future enhancements for the Australia Fire Simulation. It is designed for handoff to the Copilot coding agent. **Suppression, firefighter, and vehicle operations are NOT to be simulated in-engine; only the effects at specified positions are applied, as game engines will handle visuals and interactions.**

---

## Phase 1: Fire Retardant Physics
- Implement chemical inhibition of combustion reactions.
- Model water/foam coverage and evaporation rates.
- Suppression is applied directly at specified positions (no physics simulation of suppression agents).
- Effectiveness based on local fuel moisture and temperature.
- Add scientific references for all formulas.
- Create validation tests against known retardant effectiveness data.

## Phase 2: Advanced Weather & Atmospheric Effects
- Pyrocumulus cloud formation (fire-generated clouds).
- Atmospheric instability and fire tornadoes.
- Real-time weather data integration (optional, if data available).
- Document all scientific sources and equations used.
- Validate against observed bushfire weather phenomena.

## Phase 3: Terrain & Vegetation Integration
- Integrate digital elevation models (DEM) for terrain.
- Add vegetation mapping for fuel types and curing.
- Implement road network data for access planning (no vehicle simulation).
- Ensure terrain and vegetation data influence fire spread and behavior.
- Validate with real-world topography and vegetation datasets.

## Phase 4: Communications & Incident Command
- Simulate radio system for fireground communications.
- Model incident command structure and resource coordination.
- No simulation of firefighter or vehicle operations; only command logic and resource status.
- Add tests for command logic and coordination scenarios.

---

## Implementation Notes
- **Suppression, firefighter, and vehicle operations are NOT simulated in-engine.**
  - When suppression is applied, update the simulation state at the specified position only.
  - Game engine will handle all visual and interactive aspects.
- **Maintain scientific realism:**
  - Use published formulas and references for all new features.
  - Add validation tests for each enhancement.
- **Document all changes:**
  - Include source references, units, and scientific justification in code comments.
- **Performance:**
  - Profile new systems before optimizing; do not simplify formulas for performance unless proven necessary.

---

## References
- Rothermel Fire Spread Model (1972)
- McArthur Forest Fire Danger Index Mk5
- Byram's Fire Intensity Equations
- CSIRO Bushfire Research
- Stefan-Boltzmann Law
- WA Fire Behaviour Calculator
- Peer-reviewed literature for retardant, weather, and terrain effects

---

**Hand this file to Copilot coding agent for phased implementation.**
