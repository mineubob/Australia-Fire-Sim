//! Ultra-realistic fire simulation demo with terrain and suppression
//!
//! Demonstrates fire spread on hills with atmospheric grid, combustion physics, and suppression

use clap::Parser;
use fire_sim_core::{
    FireSimulationUltra, Fuel, FuelPart, SuppressionAgent, SuppressionDroplet, TerrainData, Vec3,
    WeatherSystem,
};

#[derive(Parser, Debug)]
#[command(name = "ultra-demo")]
#[command(about = "Ultra-realistic fire simulation demo", long_about = None)]
struct Args {
    /// Fire size: small (10 elements), medium (50), large (200), huge (500), custom
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

    /// Map width in meters (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    map_width: Option<f32>,

    /// Map height in meters (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    map_height: Option<f32>,

    /// Number of fuel elements in X direction (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    elements_x: Option<usize>,

    /// Number of fuel elements in Y direction (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    elements_y: Option<usize>,

    /// Fuel mass per element in kg (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    fuel_mass: Option<f32>,

    /// Number of elements to initially ignite (required for custom size)
    #[arg(long, required_if_eq("size", "custom"))]
    ignite_count: Option<usize>,
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

    // Get map dimensions (from args for custom, or default 200x200)
    let (map_width, map_height) = if args.size == "custom" {
        (args.map_width.unwrap(), args.map_height.unwrap())
    } else {
        (200.0, 200.0)
    };

    // Create terrain based on type
    let terrain = match args.terrain.as_str() {
        "flat" => TerrainData::flat(map_width, map_height, 5.0, 0.0),
        "valley" => TerrainData::valley_between_hills(map_width, map_height, 5.0, 0.0, 80.0),
        _ => TerrainData::single_hill(map_width, map_height, 5.0, 0.0, 80.0, 40.0),
    };
    println!(
        "✓ Created terrain: {:.0}m x {:.0}m ({})",
        map_width, map_height, args.terrain
    );

    // Create ultra-realistic simulation
    let mut sim = FireSimulationUltra::new(5.0, terrain);
    println!("✓ Created FireSimulationUltra");
    println!(
        "  - Grid: {} x {} x {} cells ({} total)",
        sim.grid.nx,
        sim.grid.ny,
        sim.grid.nz,
        sim.grid.cells.len()
    );
    println!("  - Cell size: {} m\n", sim.grid.cell_size);

    // Set weather conditions
    let weather = WeatherSystem::new(
        30.0, // 30°C temperature
        0.25, // 25% humidity
        15.0, // 15 m/s wind speed
        45.0, // 45° wind direction (NE)
        8.0,  // High drought factor
    );
    sim.set_weather(weather);
    println!("✓ Set weather conditions");
    println!("  - Temperature: 30°C");
    println!("  - Humidity: 25%");
    println!("  - Wind: 15 m/s at 45°");
    println!("  - Drought factor: 8.0 (Extreme)\n");

    // Determine fire size parameters
    let (elements_x, elements_y, fuel_mass, ignite_count) = if args.size == "custom" {
        (
            args.elements_x.unwrap(),
            args.elements_y.unwrap(),
            args.fuel_mass.unwrap(),
            args.ignite_count.unwrap(),
        )
    } else {
        match args.size.as_str() {
            "small" => (4, 4, 3.0, 2),    // 4x4 = 16 elements, 3kg each, ignite 2
            "large" => (15, 15, 8.0, 10), // 15x15 = 225 elements, 8kg each, ignite 10
            "huge" => (25, 25, 10.0, 20), // 25x25 = 625 elements, 10kg each, ignite 20
            _ => (10, 10, 5.0, 5),        // medium: 10x10 = 100 elements, 5kg each, ignite 5
        }
    };

    let total_elements = elements_x * elements_y;

    // Add fuel elements
    println!(
        "Adding {} fuel elements ({} size)...",
        total_elements, args.size
    );
    let mut fuel_ids = Vec::new();

    // Calculate center of the map
    let center_x = map_width / 2.0;
    let center_y = map_height / 2.0;

    // Calculate grid spacing to fit elements in the map
    let spacing = (map_width * 0.6 / elements_x as f32).min(map_height * 0.6 / elements_y as f32);
    let start_x = center_x - (elements_x as f32 * spacing) / 2.0;
    let start_y = center_y - (elements_y as f32 * spacing) / 2.0;

    for i in 0..elements_x {
        for j in 0..elements_y {
            let x = start_x + i as f32 * spacing;
            let y = start_y + j as f32 * spacing;
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
    println!(
        "✓ Added {} fuel elements ({:.1} kg each)\n",
        fuel_ids.len(),
        fuel_mass
    );

    // Ignite elements
    println!("Igniting fire...");
    for &fuel_id in fuel_ids.iter().take(ignite_count) {
        sim.ignite_element(fuel_id, 600.0);
    }
    println!("✓ Ignited {} elements at 600°C\n", ignite_count);

    // Run simulation
    println!("Running simulation for {}s...\n", args.duration);
    println!("Time | Burning | Active Cells | Max Temp | Fuel Consumed |  FPS  | Update Time");
    println!("-----|---------|--------------|----------|---------------|-------|------------");

    let suppression_time = args.duration / 2;
    let mut total_update_time = 0.0;
    let mut update_count = 0;

    for step in 0..args.duration {
        let t = step as f32;

        // Measure update performance
        let start_time = std::time::Instant::now();
        sim.update(1.0); // 1 second timestep
        let update_time = start_time.elapsed();

        total_update_time += update_time.as_secs_f32();
        update_count += 1;

        let stats = sim.get_stats();

        // Find max temperature in grid
        let max_temp = sim
            .grid
            .cells
            .iter()
            .map(|c| c.temperature)
            .fold(0.0f32, f32::max);

        // Calculate instantaneous FPS
        let fps = 1.0 / update_time.as_secs_f64();

        // Print every 2 seconds or at key moments
        if step % 2 == 0 || step == suppression_time {
            println!(
                "{:4.0}s | {:7} | {:12} | {:7.0}°C | {:10.2} kg | {:5.1} | {:9.1} ms",
                t,
                stats.burning_elements,
                stats.active_cells,
                max_temp,
                stats.total_fuel_consumed,
                fps,
                update_time.as_secs_f64() * 1000.0
            );
        }

        // Add suppression at halfway point
        if args.suppression && step == suppression_time {
            println!("\n>>> DEPLOYING WATER SUPPRESSION <<<\n");

            // Add water droplets in a circular pattern around the center
            for i in 0..30 {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
                let radius = 25.0;
                let droplet = SuppressionDroplet::new(
                    Vec3::new(
                        center_x + angle.cos() * radius,
                        center_y + angle.sin() * radius,
                        60.0, // 60m altitude
                    ),
                    Vec3::new(0.0, 0.0, -5.0), // Falling downward
                    10.0,                      // 10 kg each
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
    println!(
        "  - Total fuel consumed: {:.2} kg",
        final_stats.total_fuel_consumed
    );
    println!(
        "  - Peak burning elements: {} elements",
        final_stats.burning_elements
    );
    println!(
        "  - Active grid cells: {} / {}",
        final_stats.active_cells, final_stats.total_cells
    );

    println!("\nPerformance Metrics:");
    let avg_update_time_ms = (total_update_time / update_count as f32) * 1000.0;
    let avg_fps = 1000.0 / avg_update_time_ms;
    println!("  - Average update time: {:.2} ms", avg_update_time_ms);
    println!("  - Average FPS: {:.1}", avg_fps);
    println!("  - Total simulation time: {:.2} s", total_update_time);
    println!(
        "  - Grid efficiency: {:.1}% cells active",
        100.0 * final_stats.active_cells as f32 / final_stats.total_cells as f32
    );
    println!(
        "  - Active cells: {} / {} ({:.1}%)",
        final_stats.active_cells,
        final_stats.total_cells,
        final_stats.active_cells as f32 / final_stats.total_cells as f32 * 100.0
    );
    println!("  - Simulation time: {:.1}s", final_stats.simulation_time);

    // Analyze atmospheric effects
    println!("\nAtmospheric Analysis:");
    let center_pos = Vec3::new(center_x, center_y, 20.0);
    if let Some(cell) = sim.get_cell_at_position(center_pos) {
        println!("  - Cell at ({:.0}, {:.0}, 20m):", center_x, center_y);
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
