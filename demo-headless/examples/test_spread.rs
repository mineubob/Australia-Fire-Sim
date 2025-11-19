// Test fire spread with minimal setup
use fire_sim_core::{FireSimulation, Fuel, FuelPart, TerrainData, Vec3, WeatherSystem};

fn main() {
    // Create small flat terrain
    let terrain = TerrainData::flat(50.0, 50.0, 5.0, 0.0);
    let mut sim = FireSimulation::new(5.0, terrain);

    // Set weather (hot and dry for good spread)
    let weather = WeatherSystem::new(35.0, 0.15, 5.0, 0.0, 10.0);
    sim.set_weather(weather);

    // Add 3 elements in a line, 5m apart (well within 15m search radius)
    let fuel = Fuel::dry_grass();
    let _id1 = sim.add_fuel_element(
        Vec3::new(20.0, 25.0, 1.0),
        fuel.clone(),
        3.0,
        FuelPart::GroundVegetation,
        None,
    );
    let _id2 = sim.add_fuel_element(
        Vec3::new(25.0, 25.0, 1.0),
        fuel.clone(),
        3.0,
        FuelPart::GroundVegetation,
        None,
    );
    let _id3 = sim.add_fuel_element(
        Vec3::new(30.0, 25.0, 1.0),
        fuel.clone(),
        3.0,
        FuelPart::GroundVegetation,
        None,
    );

    println!("Created 3 fuel elements 5m apart");
    println!("Element 1 at (20, 25, 1)");
    println!("Element 2 at (25, 25, 1)");
    println!("Element 3 at (30, 25, 1)");

    // Ignite only the first element
    sim.ignite_element(_id1, 600.0);
    println!("\nIgnited element 1 at 600°C");

    // Run simulation for 30 seconds
    println!("\nTime | Burning | Active Cells | Grid@E1 | Grid@E2 | Grid@E3");
    println!("-----|---------|--------------|---------|---------|--------");

    for step in 0..30 {
        sim.update(1.0);

        let grid_temp1 = sim
            .get_cell_at_position(Vec3::new(20.0, 25.0, 1.0))
            .map(|c| c.temperature())
            .unwrap_or(0.0);
        let grid_temp2 = sim
            .get_cell_at_position(Vec3::new(25.0, 25.0, 1.0))
            .map(|c| c.temperature())
            .unwrap_or(0.0);
        let grid_temp3 = sim
            .get_cell_at_position(Vec3::new(30.0, 25.0, 1.0))
            .map(|c| c.temperature())
            .unwrap_or(0.0);

        if step % 2 == 0 {
            let stats = sim.get_stats();
            println!(
                "{:4}s | {:7} | {:12} | {:7.0}° | {:7.0}° | {:7.0}°",
                step,
                stats.burning_elements,
                stats.active_cells,
                grid_temp1,
                grid_temp2,
                grid_temp3
            );
        }
    }

    println!("\nFinal burning: {}", sim.get_stats().burning_elements);
    println!("Expected: 3 (if fire spread worked)");
    println!("Actual: {}", sim.get_stats().burning_elements);

    if sim.get_stats().burning_elements >= 2 {
        println!("\n✓ FIRE SPREAD WORKING!");
    } else {
        println!("\n✗ Fire did not spread beyond initial ignition");
    }
}
