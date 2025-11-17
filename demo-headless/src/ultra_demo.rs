//! Ultra-realistic fire simulation demo with terrain and suppression
//! 
//! Demonstrates fire spread on hills with atmospheric grid, combustion physics, and suppression

use fire_sim_core::{
    FireSimulationUltra, TerrainData, Fuel, FuelPart, Vec3, WeatherSystem,
    SuppressionDroplet, SuppressionAgent,
};

fn main() {
    println!("========================================");
    println!("ULTRA-REALISTIC FIRE SIMULATION DEMO");
    println!("========================================\n");
    
    // Create terrain with a hill
    let terrain = TerrainData::single_hill(200.0, 200.0, 5.0, 0.0, 80.0, 40.0);
    println!("✓ Created terrain: 200m x 200m with 80m hill");
    println!("  - Peak at center (100m, 100m)");
    println!("  - Base elevation: 0m");
    println!("  - Peak elevation: ~80m\n");
    
    // Create ultra-realistic simulation
    let mut sim = FireSimulationUltra::new(5.0, terrain);
    println!("✓ Created FireSimulationUltra");
    println!("  - Grid: {} x {} x {} cells ({} total)",
             sim.grid.nx, sim.grid.ny, sim.grid.nz, sim.grid.cells.len());
    println!("  - Cell size: {} m", sim.grid.cell_size);
    println!("  - Total volume: {:.0} m³\n", 200.0 * 200.0 * 100.0);
    
    // Set weather conditions
    let weather = WeatherSystem::new(
        30.0,  // 30°C temperature
        0.25,  // 25% humidity
        15.0,  // 15 m/s wind speed
        45.0,  // 45° wind direction (NE)
        8.0,   // High drought factor
    );
    sim.set_weather(weather);
    println!("✓ Set weather conditions");
    println!("  - Temperature: 30°C");
    println!("  - Humidity: 25%");
    println!("  - Wind: 15 m/s at 45°");
    println!("  - Drought factor: 8.0 (Extreme)\n");
    
    // Add fuel elements on the hillside
    println!("Adding fuel elements on hillside...");
    let mut fuel_ids = Vec::new();
    
    // Create a patch of dry grass near the base of the hill
    for i in 0..10 {
        for j in 0..10 {
            let x = 80.0 + i as f32 * 3.0;
            let y = 80.0 + j as f32 * 3.0;
            let elevation = sim.grid.terrain.elevation_at(x, y);
            
            let fuel = Fuel::dry_grass();
            let id = sim.add_fuel_element(
                Vec3::new(x, y, elevation + 0.5),
                fuel,
                5.0,  // 5 kg per element - more fuel for longer burning
                FuelPart::GroundVegetation,
                None,
            );
            fuel_ids.push(id);
        }
    }
    println!("✓ Added {} fuel elements (dry grass)\n", fuel_ids.len());
    
    // Ignite a few elements at the base of the hill
    println!("Igniting fire at base of hill...");
    for i in 0..3 {
        sim.ignite_element(fuel_ids[i], 600.0);
    }
    println!("✓ Ignited {} elements at 600°C\n", 3);
    
    // Run simulation
    println!("Running simulation...\n");
    println!("Time | Burning | Active Cells | Max Temp | Fuel Consumed");
    println!("-----|---------|--------------|----------|---------------");
    
    for step in 0..30 {
        let t = step as f32;
        sim.update(1.0);  // 1 second timestep
        
        let stats = sim.get_stats();
        
        // Find max temperature in grid
        let max_temp = sim.grid.cells.iter()
            .map(|c| c.temperature)
            .fold(0.0f32, f32::max);
        
        if step % 2 == 0 {
            println!("{:4.0}s | {:7} | {:12} | {:7.0}°C | {:7.2} kg",
                     t, stats.burning_elements, stats.active_cells,
                     max_temp, stats.total_fuel_consumed);
        }
        
        // Add suppression at t=15s
        if step == 15 {
            println!("\n>>> DEPLOYING WATER SUPPRESSION <<<\n");
            
            // Add water droplets in a pattern
            for i in 0..20 {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / 20.0;
                let droplet = SuppressionDroplet::new(
                    Vec3::new(
                        100.0 + angle.cos() * 20.0,
                        100.0 + angle.sin() * 20.0,
                        50.0,  // 50m altitude
                    ),
                    Vec3::new(0.0, 0.0, -5.0),  // Falling downward
                    5.0,  // 5 kg each
                    SuppressionAgent::Water,
                );
                sim.add_suppression_droplet(droplet);
            }
        }
    }
    
    println!("\n========================================");
    println!("SIMULATION COMPLETE");
    println!("========================================\n");
    
    let final_stats = sim.get_stats();
    println!("Final Statistics:");
    println!("  - Total fuel consumed: {:.2} kg", final_stats.total_fuel_consumed);
    println!("  - Peak burning elements: {} elements", final_stats.burning_elements);
    println!("  - Active cells: {} / {} ({:.1}%)",
             final_stats.active_cells,
             final_stats.total_cells,
             final_stats.active_cells as f32 / final_stats.total_cells as f32 * 100.0);
    println!("  - Simulation time: {:.1}s", final_stats.simulation_time);
    
    // Analyze atmospheric effects
    println!("\nAtmospheric Analysis:");
    let center_pos = Vec3::new(100.0, 100.0, 20.0);
    if let Some(cell) = sim.get_cell_at_position(center_pos) {
        println!("  - Cell at (100, 100, 20m):");
        println!("    Temperature: {:.1}°C", cell.temperature);
        println!("    Oxygen: {:.3} kg/m³", cell.oxygen);
        println!("    Smoke: {:.4} kg/m³", cell.smoke_particles);
        println!("    CO2: {:.4} kg/m³", cell.carbon_dioxide);
    }
    
    println!("\n✓ Demo complete! The simulation demonstrated:");
    println!("  1. Fire spread on terrain with elevation");
    println!("  2. Atmospheric grid with combustion physics");
    println!("  3. Oxygen depletion and smoke generation");
    println!("  4. Water suppression effects");
    println!("  5. Buoyancy-driven plume formation");
}
