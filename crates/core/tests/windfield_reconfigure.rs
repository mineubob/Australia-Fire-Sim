use fire_sim_core::{FireSimulation, TerrainData, Vec3, WindFieldConfig};

#[test]
fn test_reconfigure_wind_field() {
    // Create a small simulation and reconfigure the always-present wind field
    let mut sim = FireSimulation::new(5.0, &TerrainData::flat(100.0, 100.0, 5.0, 0.0));

    // Make a small, low-resolution config and apply it
    let cfg = WindFieldConfig {
        nx: 5,
        ny: 5,
        nz: 8,
        ..Default::default()
    };
    sim.reconfigure_wind_field(cfg);

    // Query wind at a sample position to ensure the field is responsive
    let w = sim.wind_at_position(Vec3::new(10.0, 10.0, 10.0));
    assert!(w.x.is_finite() && w.y.is_finite() && w.z.is_finite());
}
