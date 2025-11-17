//! Ultra-realistic fire simulation demo with terrain and suppression
//! 
//! Demonstrates fire spread on hills with atmospheric grid, combustion physics, and suppression

use fire_sim_core::{
    FireSimulationUltra, TerrainData, Fuel, FuelPart, Vec3, WeatherSystem,
    SuppressionDroplet, SuppressionAgent,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ultra-demo")]
#[command(about = "Ultra-realistic fire simulation demo", long_about = None)]
struct Args {
    /// Fire size: small (10 elements), medium (50), large (200), huge (500)
    #[arg(short, long, default_value = "medium")]
    size: String,
    
    /// Simulation duration in seconds
    #[arg(short, long, default_value = "60")]
    duration: u32,
    
    /// Terrain type: flat, hill, valley
    #[arg(short, long, default_value = "hill")]
    terrain: String,
    
    /// Enable water suppression at halfway point
    #[arg(short = 'w', long, default_value = "true")]
    suppression: bool,
}

fn main() {
    let args = Args::parse();
    println!("========================================");
    println!("ULTRA-REALISTIC FIRE SIMULATION DEMO");
    println!("========================================");
    println!("Configuration:");
    println!("  - Fire size: {}", args.size);
    println!("  - Duration: {}s", args.duration);
    println!("  - Terrain: {}", args.terrain);
    println!("  - Suppression: {}\n", args.suppression);
    
    // Create terrain based on type
    let terrain = match args.terrain.as_str() {
        "flat" => TerrainData::flat(200.0, 200.0, 5.0, 0.0),
        "valley" => TerrainData::valley_between_hills(200.0, 200.0, 5.0, 0.0, 80.0),
        _ => TerrainData::single_hill(200.0, 200.0, 5.0, 0.0, 80.0, 40.0),
    };
    println!("✓ Created terrain: 200m x 200m ({})", args.terrain);
    
    // Create ultra-realistic simulation
    let mut sim = FireSimulationUltra::new(5.0, terrain);
    println!("✓ Created FireSimulationUltra");
    println!("  - Grid: {} x {} x {} cells ({} total)",
             sim.grid.nx, sim.grid.ny, sim.grid.nz, sim.grid.cells.len());
    println!("  - Cell size: {} m\n", sim.grid.cell_size);
    
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
    
    // Determine fire size parameters
    let (grid_size, fuel_mass, ignite_count) = match args.size.as_str() {
        "small" => (4, 3.0, 2),      // 4x4 = 16 elements, 3kg each, ignite 2
        "large" => (15, 8.0, 10),    // 15x15 = 225 elements, 8kg each, ignite 10
        "huge" => (25, 10.0, 20),    // 25x25 = 625 elements, 10kg each, ignite 20
        _ => (10, 5.0, 5),           // medium: 10x10 = 100 elements, 5kg each, ignite 5
    };
    
    // Add fuel elements
    println!("Adding {} fuel elements ({} size)...", grid_size * grid_size, args.size);
    let mut fuel_ids = Vec::new();
    
    let start_x = 100.0 - (grid_size as f32 * 1.5);
    let start_y = 100.0 - (grid_size as f32 * 1.5);
    
    for i in 0..grid_size {
        for j in 0..grid_size {
            let x = start_x + i as f32 * 3.0;
            let y = start_y + j as f32 * 3.0;
            let elevation = sim.grid.terrain.elevation_at(x, y);
            
            let fuel = Fuel::dry_grass();
            let id = sim.add_fuel_element(
                Vec3::new(x, y, elevation + 0.5),
                fuel,
                fuel_mass,
                FuelPart::GroundVegetation,
                None,
            );
            fuel_ids.push(id);
        }
    }
    println!("✓ Added {} fuel elements ({:.1} kg each)\n", fuel_ids.len(), fuel_mass);
    
    // Ignite elements
    println!("Igniting fire...");
    for i in 0..ignite_count.min(fuel_ids.len()) {
        sim.ignite_element(fuel_ids[i], 600.0);
    }
    println!("✓ Ignited {} elements at 600°C\n", ignite_count);
    
    // Run simulation
    println!("Running simulation for {}s...\n", args.duration);
    println!("Time | Burning | Active Cells | Max Temp | Fuel Consumed");
    println!("-----|---------|--------------|----------|---------------");
    
    let suppression_time = args.duration / 2;
    
    for step in 0..args.duration {
        let t = step as f32;
        sim.update(1.0);  // 1 second timestep
        
        let stats = sim.get_stats();
        
        // Find max temperature in grid
        let max_temp = sim.grid.cells.iter()
            .map(|c| c.temperature)
            .fold(0.0f32, f32::max);
        
        // Print every 2 seconds or at key moments
        if step % 2 == 0 || step == suppression_time {
            println!("{:4.0}s | {:7} | {:12} | {:7.0}°C | {:7.2} kg",
                     t, stats.burning_elements, stats.active_cells,
                     max_temp, stats.total_fuel_consumed);
        }
        
        // Add suppression at halfway point
        if args.suppression && step == suppression_time {
            println!("\n>>> DEPLOYING WATER SUPPRESSION <<<\n");
            
            // Add water droplets in a circular pattern
            for i in 0..30 {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
                let radius = 25.0;
                let droplet = SuppressionDroplet::new(
                    Vec3::new(
                        100.0 + angle.cos() * radius,
                        100.0 + angle.sin() * radius,
                        60.0,  // 60m altitude
                    ),
                    Vec3::new(0.0, 0.0, -5.0),  // Falling downward
                    10.0,  // 10 kg each
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
