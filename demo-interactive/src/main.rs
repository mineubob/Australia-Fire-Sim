//! Interactive Fire Simulation Demo
//!
//! A terminal-based interactive debugger for the fire simulation.
//! Allows stepping through the simulation, inspecting element values,
//! and debugging fire spread behavior.
//!
//! # Usage
//!
//! ```bash
//! cargo run --package demo-interactive
//! ```
//!
//! # Commands
//!
//! - `step [n]` - Advance simulation by n timesteps (default 1)
//! - `status` - Show simulation status
//! - `weather` - Show current weather conditions
//! - `element <id>` - Show element details
//! - `burning` - List all burning elements
//! - `embers` - List all active embers
//! - `nearby <id>` - Show elements near the specified element
//! - `ignite <id>` - Manually ignite an element
//! - `ignite_position <x> <y> [radius] [amount]` - Ignite elements around position in XY circle
//!   - radius: optional, meters (default 1.0)
//!   - amount: optional, number of elements to ignite (from ground-up). Use -1 for all (default -1)
//!   - filters: optional tokens to further limit selection: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>
//! - `heat <id> <temperature>` - Apply heat to an element (target temp in °C)
//! - `heat_position <x> <y> <temperature> [radius] [amount]` - Apply heat to elements around position
//!   - temperature: target temperature in Celsius
//!   - radius: optional, meters (default 1.0)
//!   - amount: optional, number of elements to heat (from ground-up). Use -1 for all (default -1)
//!   - filters: optional tokens to further limit selection: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>
//! - `preset <name>` - Switch weather preset (perth, catastrophic, etc.)
//! - `reset` - Reset simulation with new terrain dimensions
//! - `heatmap [size]` - Generate a heatmap of the simulation
//! - `help` - Show available commands
//! - `quit` - Exit the simulation

use fire_sim_core::{
    ClimatePattern, FireSimulation, Fuel, FuelPart, TerrainData, Vec3, WeatherPreset, WeatherSystem,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::{
    io::{self, Write},
    time::Instant,
};

/// Default terrain dimensions
const DEFAULT_WIDTH: f32 = 150.0;
const DEFAULT_HEIGHT: f32 = 150.0;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      Australia Fire Simulation - Interactive Debugger     ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Ask for terrain dimensions
    let (width, height) = prompt_terrain_dimensions();

    // Create simulation with user-specified dimensions
    let mut sim = create_test_simulation(width, height);
    let mut current_width = width;
    let mut current_height = height;

    println!(
        "Created simulation with {} elements on {}x{} terrain",
        sim.get_all_elements().len(),
        width,
        height
    );
    println!("No elements are ignited. Use 'ignite <id>' to start a fire.");

    // Setup readline
    let mut rl = match DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("Failed to create readline: {}", e);
            return;
        }
    };

    println!("\nType 'help' for available commands.\n");

    loop {
        let readline = rl.readline("fire> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);
                let parts: Vec<&str> = line.split_whitespace().collect();

                if parts.is_empty() {
                    continue;
                }

                match parts[0].to_lowercase().as_str() {
                    "step" | "s" => {
                        let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                        step_simulation(&mut sim, count);
                    }
                    "status" | "st" => show_status(&sim),
                    "weather" | "w" => show_weather(&sim),
                    "element" | "e" => {
                        if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                            show_element(&sim, id);
                        } else {
                            println!("Usage: element <id>");
                        }
                    }
                    "burning" | "b" => show_burning(&sim),
                    "embers" | "em" => show_embers(&sim),
                    "nearby" | "n" => {
                        if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                            show_nearby(&sim, id);
                        } else {
                            println!("Usage: nearby <id>");
                        }
                    }
                    "ignite" | "i" => {
                        if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                            ignite_element(&mut sim, id);
                        } else {
                            println!("Usage: ignite <id>");
                        }
                    }
                    "heat" | "h" => {
                        if let (Some(id), Some(temperature)) = (
                            parts.get(1).and_then(|s| s.parse().ok()),
                            parts.get(2).and_then(|s| s.parse().ok()),
                        ) {
                            heat_element(&mut sim, id, temperature);
                        } else {
                            println!("Usage: heat <id> <temperature>");
                        }
                    }
                    "ignite_position" | "ip" => {
                        let Some(x) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) else {
                            println!("Usage: ignite_position <x> <y> [radius] [amount] [filters]  (radius default=1.0, amount -1 = all)");
                            println!(
                                "Filters: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>"
                            );
                            continue;
                        };

                        let Some(y) = parts.get(2).and_then(|s| s.parse::<i32>().ok()) else {
                            println!("Usage: ignite_position <x> <y> [radius] [amount] [filters]  (radius default=1.0, amount -1 = all)");
                            println!(
                                "Filters: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>"
                            );
                            continue;
                        };

                        // Parse optional radius and amount parameters
                        // Usage: ignite_position <x> <y> [radius] [amount]
                        // amount: number of elements to ignite (from ground up). -1 = all
                        let radius = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                        let amount = parts
                            .get(4)
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(-1);

                        // Parse optional filters after amount
                        let (fuel_filter, part_filter, min_z, max_z) = parse_filters(&parts, 5);

                        let center = Vec3::new(x as f32, y as f32, 0.0);

                        // Get filtered elements with ignition temperatures
                        let filtered = filter_elements_in_circle(
                            &sim,
                            center,
                            radius,
                            fuel_filter,
                            part_filter,
                            min_z,
                            max_z,
                        );

                        let mut id_dist_ign: Vec<(u32, f32, f32, f32)> = filtered
                            .into_iter()
                            .filter_map(|(id, dist, z)| {
                                sim.get_element(id)
                                    .map(|e| (id, dist, e.fuel().ignition_temperature, z))
                            })
                            .collect();

                        if id_dist_ign.is_empty() {
                            println!(
                                "No elements found within radius {:.1} at ({}, {})",
                                radius, x, y
                            );
                        } else {
                            // Sort by Z ascending (ground-up), then horizontal distance ascending
                            id_dist_ign.sort_by(|a, b| {
                                let z_cmp =
                                    a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal);
                                if z_cmp == std::cmp::Ordering::Equal {
                                    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                                } else {
                                    z_cmp
                                }
                            });

                            let total = id_dist_ign.len();
                            let to_ignite: Vec<(u32, f32, f32, f32)> = if amount < 0 {
                                id_dist_ign.clone()
                            } else {
                                let amt = amount as usize;
                                id_dist_ign.into_iter().take(amt).collect()
                            };

                            println!(
                                "Found {} element(s) within radius {:.1} — igniting {} (ground-up → closest):",
                                total,
                                radius,
                                if amount < 0 { total } else { to_ignite.len() }
                            );

                            for (id, dist, ign_temp, z) in to_ignite.iter() {
                                println!(
                                    "  ID {}: {:.2}m, z={:.2} — ignition temp {:.1}°C",
                                    id, dist, z, ign_temp
                                );
                                // Start at 600°C - realistic for piloted ignition
                                let initial_temp = 600.0_f32.max(*ign_temp);
                                sim.ignite_element(*id, initial_temp);
                            }
                        }
                    }
                    "heat_position" | "hp" => {
                        let Some(x) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) else {
                            println!("Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]  (radius default=1.0, amount -1 = all)");
                            println!(
                                "Filters: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>"
                            );
                            continue;
                        };

                        let Some(y) = parts.get(2).and_then(|s| s.parse::<i32>().ok()) else {
                            println!("Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]  (radius default=1.0, amount -1 = all)");
                            println!(
                                "Filters: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>"
                            );
                            continue;
                        };

                        let Some(temperature) = parts.get(3).and_then(|s| s.parse::<f32>().ok())
                        else {
                            println!("Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]  (radius default=1.0, amount -1 = all)");
                            println!(
                                "Filters: fuel=<name>, part=<partname>, minz=<f32>, maxz=<f32>"
                            );
                            continue;
                        };

                        // Parse optional radius and amount parameters
                        let radius = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                        let amount = parts
                            .get(5)
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(-1);

                        // Parse optional filters after amount
                        let (fuel_filter, part_filter, min_z, max_z) = parse_filters(&parts, 6);

                        let center = Vec3::new(x as f32, y as f32, 0.0);

                        // Get filtered elements
                        let mut id_dist_z = filter_elements_in_circle(
                            &sim,
                            center,
                            radius,
                            fuel_filter,
                            part_filter,
                            min_z,
                            max_z,
                        );

                        if id_dist_z.is_empty() {
                            println!(
                                "No elements found within radius {:.1} at ({}, {})",
                                radius, x, y
                            );
                        } else {
                            // Sort by Z ascending (ground-up), then horizontal distance ascending
                            id_dist_z.sort_by(|a, b| {
                                let z_cmp =
                                    a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
                                if z_cmp == std::cmp::Ordering::Equal {
                                    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                                } else {
                                    z_cmp
                                }
                            });

                            let total = id_dist_z.len();
                            let to_heat: Vec<(u32, f32, f32)> = if amount < 0 {
                                id_dist_z.clone()
                            } else {
                                let amt = amount as usize;
                                id_dist_z.into_iter().take(amt).collect()
                            };

                            println!(
                                "Found {} element(s) within radius {:.1} — heating {} to {:.1}°C (ground-up → closest):",
                                total,
                                radius,
                                if amount < 0 { total } else { to_heat.len() },
                                temperature
                            );

                            for (id, dist, z) in to_heat.iter() {
                                println!("  ID {}: {:.2}m, z={:.2}", id, dist, z);
                                heat_element_to_temp(&mut sim, *id, temperature);
                            }
                        }
                    }
                    "preset" | "p" => {
                        if let Some(name) = parts.get(1) {
                            set_preset(&mut sim, name);
                        } else {
                            println!("Usage: preset <perth|catastrophic|goldfields|wheatbelt>");
                        }
                    }
                    "reset" | "r" => {
                        // Parse optional dimensions from command or use current
                        let new_width = parts
                            .get(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(current_width);
                        let new_height = parts
                            .get(2)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(current_height);

                        sim = create_test_simulation(new_width, new_height);
                        current_width = new_width;
                        current_height = new_height;

                        println!(
                            "Simulation reset! Created {} elements on {}x{} terrain",
                            sim.get_all_elements().len(),
                            new_width,
                            new_height
                        );
                    }
                    "help" | "?" => show_help(),
                    "heatmap" | "hm" => {
                        let grid_size = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);

                        show_heatmap(&sim, current_width, current_height, grid_size)
                    }
                    "quit" | "q" | "exit" => {
                        println!("Goodbye!");
                        break;
                    }
                    _ => println!(
                        "Unknown command: {}. Type 'help' for available commands.",
                        parts[0]
                    ),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}

/// Prompt user for terrain dimensions at startup
fn prompt_terrain_dimensions() -> (f32, f32) {
    println!("Enter terrain dimensions (or press Enter for defaults):");

    // Width
    print!("  Width in meters [{}]: ", DEFAULT_WIDTH);
    io::stdout().flush().unwrap();
    let mut width_str = String::new();
    io::stdin().read_line(&mut width_str).unwrap();
    let width: f32 = width_str.trim().parse().unwrap_or(DEFAULT_WIDTH);

    // Height
    print!("  Height in meters [{}]: ", DEFAULT_HEIGHT);
    io::stdout().flush().unwrap();
    let mut height_str = String::new();
    io::stdin().read_line(&mut height_str).unwrap();
    let height: f32 = height_str.trim().parse().unwrap_or(DEFAULT_HEIGHT);

    // Validate dimensions
    let width = width.clamp(10.0, 1000.0);
    let height = height.clamp(10.0, 1000.0);

    println!();
    (width, height)
}

/// Parse filter tokens from command arguments
fn parse_filters(
    parts: &[&str],
    start_idx: usize,
) -> (Option<String>, Option<String>, Option<f32>, Option<f32>) {
    let mut fuel_filter: Option<String> = None;
    let mut part_filter: Option<String> = None;
    let mut min_z: Option<f32> = None;
    let mut max_z: Option<f32> = None;

    for token in parts.iter().skip(start_idx) {
        if let Some((key, val)) = token.split_once('=') {
            match key.to_lowercase().as_str() {
                "fuel" => fuel_filter = Some(val.to_lowercase()),
                "part" => part_filter = Some(val.to_lowercase()),
                "minz" => min_z = val.parse::<f32>().ok(),
                "maxz" => max_z = val.parse::<f32>().ok(),
                _ => {
                    println!(
                        "Unknown filter '{}', supported: fuel=, part=, minz=, maxz=",
                        key
                    );
                }
            }
        }
    }

    (fuel_filter, part_filter, min_z, max_z)
}

/// Get part name as a string for filtering
fn get_part_name(part: &fire_sim_core::core_types::element::FuelPart) -> String {
    match part {
        fire_sim_core::core_types::element::FuelPart::Root => "root".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkLower => "trunklower".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkMiddle => "trunkmiddle".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkUpper => "trunkupper".to_string(),
        fire_sim_core::core_types::element::FuelPart::BarkLayer(h) => {
            format!("barklayer({:.0})", h)
        }
        fire_sim_core::core_types::element::FuelPart::Branch { height, angle: _ } => {
            format!("branch({:.0})", height)
        }
        fire_sim_core::core_types::element::FuelPart::Crown => "crown".to_string(),
        fire_sim_core::core_types::element::FuelPart::GroundLitter => "groundlitter".to_string(),
        fire_sim_core::core_types::element::FuelPart::GroundVegetation => {
            "groundvegetation".to_string()
        }
        fire_sim_core::core_types::element::FuelPart::BuildingWall { floor } => {
            format!("buildingwall({})", floor)
        }
        fire_sim_core::core_types::element::FuelPart::BuildingRoof => "buildingroof".to_string(),
        fire_sim_core::core_types::element::FuelPart::BuildingInterior => {
            "buildinginterior".to_string()
        }
        fire_sim_core::core_types::element::FuelPart::Vehicle => "vehicle".to_string(),
        fire_sim_core::core_types::element::FuelPart::Surface => "surface".to_string(),
    }
}

/// Filter elements within a 2D circle radius, applying optional fuel/part/z filters
fn filter_elements_in_circle(
    sim: &FireSimulation,
    center: Vec3,
    radius: f32,
    fuel_filter: Option<String>,
    part_filter: Option<String>,
    min_z: Option<f32>,
    max_z: Option<f32>,
) -> Vec<(u32, f32, f32)> {
    let candidates = sim.get_elements_in_radius(center, radius);

    candidates
        .into_iter()
        .filter_map(|e| {
            let dx = e.position().x - center.x;
            let dy = e.position().y - center.y;
            let dist2d = (dx * dx + dy * dy).sqrt();

            if dist2d <= radius {
                // Apply fuel filter
                if let Some(ref f) = fuel_filter {
                    let fuel_name = e.fuel().name.to_lowercase();
                    if !fuel_name.contains(f) {
                        return None;
                    }
                }

                // Apply part filter
                if let Some(ref p) = part_filter {
                    let part_name = get_part_name(&e.part_type());
                    if !part_name.to_lowercase().contains(p) {
                        return None;
                    }
                }

                // Apply min z filter
                if let Some(minz) = min_z {
                    if e.position().z < minz {
                        return None;
                    }
                }

                // Apply max z filter
                if let Some(maxz) = max_z {
                    if e.position().z > maxz {
                        return None;
                    }
                }

                Some((e.id(), dist2d, e.position().z))
            } else {
                None
            }
        })
        .collect()
}

fn create_test_simulation(width: f32, height: f32) -> FireSimulation {
    let mut sim = FireSimulation::new(5.0, TerrainData::flat(width, height, 5.0, 0.0));

    // Create a grid of fuel elements representing different vegetation
    // Ground layer: grass and shrubs
    // 
    // SPACING: 1m simulates near-continuous fuel beds like real grasslands.
    // Fire spreads through direct flame contact and short-range radiation.
    // Smaller spacing = faster, more realistic spread patterns.
    let step = 1;
    for x in (0..(width as i32)).step_by(step) {
        for y in (0..(height as i32)).step_by(step) {
            let fuel = if (x + y) % 20 == 0 {
                Fuel::dead_wood_litter()
            } else {
                Fuel::dry_grass()
            };

            let id = sim.add_fuel_element(
                Vec3::new(x as f32, y as f32, 0.0),
                fuel,
                3.0,
                FuelPart::GroundVegetation,
                None,
            );

            // Add some trees (every 15m)
            if x % 15 == 0 && y % 15 == 0 {
                create_tree(&mut sim, x as f32, y as f32, id);
            }
        }
    }

    // Set to Perth Metro conditions
    let weather = WeatherSystem::from_preset(
        WeatherPreset::perth_metro(),
        3,    // January 3
        14.0, // 2pm
        ClimatePattern::Neutral,
    );
    sim.set_weather(weather);

    sim
}

fn create_tree(sim: &mut FireSimulation, x: f32, y: f32, _ground_id: u32) {
    // Trunk
    let trunk_id = sim.add_fuel_element(
        Vec3::new(x, y, 2.0),
        Fuel::eucalyptus_stringybark(),
        10.0,
        FuelPart::TrunkLower,
        None,
    );

    // Lower branches
    sim.add_fuel_element(
        Vec3::new(x - 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        3.0,
        FuelPart::Branch {
            height: 4.0,
            angle: 0.0,
        },
        Some(trunk_id),
    );
    sim.add_fuel_element(
        Vec3::new(x + 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        3.0,
        FuelPart::Branch {
            height: 4.0,
            angle: 180.0,
        },
        Some(trunk_id),
    );

    // Crown
    sim.add_fuel_element(
        Vec3::new(x, y, 8.0),
        Fuel::eucalyptus_stringybark(),
        5.0,
        FuelPart::Crown,
        Some(trunk_id),
    );
}

fn step_simulation(sim: &mut FireSimulation, count: u32) {
    let dt = 1.0; // 1 second timestep
    println!("Stepping {} timestep(s)...", count);

    for i in 0..count {
        let burning_before = sim.get_burning_elements().len();
        let embers_before = sim.ember_count();
        let start = Instant::now();

        sim.update(dt);

        let burning_after = sim.get_burning_elements().len();
        let embers_after = sim.ember_count();
        let time = start.elapsed();

        if i == count - 1 || burning_after != burning_before || embers_after != embers_before {
            println!(
                "  Step {}: Burning: {} → {}, Embers: {} → {}, Time: {}ms",
                i + 1,
                burning_before,
                burning_after,
                embers_before,
                embers_after,
                time.as_millis()
            );
        }
    }
    println!("Done.");
}

fn show_status(sim: &FireSimulation) {
    println!("\n═══════════════ SIMULATION STATUS ═══════════════");
    println!("Total elements:    {}", sim.get_all_elements().len());
    println!("Burning elements:  {}", sim.get_burning_elements().len());
    println!("Active embers:     {}", sim.ember_count());

    // Find temperature range of burning elements
    let burning: Vec<_> = sim
        .get_all_elements()
        .iter()
        .map(|e| e.get_stats())
        .collect();
    if !burning.is_empty() {
        let min_temp = burning
            .iter()
            .map(|e| e.temperature)
            .fold(f32::MAX, f32::min);
        let max_temp = burning
            .iter()
            .map(|e| e.temperature)
            .fold(f32::MIN, f32::max);
        let avg_temp: f32 =
            burning.iter().map(|e| e.temperature).sum::<f32>() / burning.len() as f32;

        println!("\nBurning element temperatures:");
        println!("  Min: {:.1}°C", min_temp);
        println!("  Max: {:.1}°C", max_temp);
        println!("  Avg: {:.1}°C", avg_temp);
    }

    println!("══════════════════════════════════════════════════\n");
}

fn show_weather(sim: &FireSimulation) {
    let w = sim.get_weather().get_stats();
    println!("\n═══════════════ WEATHER CONDITIONS ═══════════════");
    println!("Temperature:     {:.1}°C", w.temperature);
    println!("Humidity:        {:.1}%", w.humidity);
    println!(
        "Wind Speed:      {:.1} km/h ({:.1} m/s)",
        w.wind_speed,
        w.wind_speed / 3.6
    );
    println!("Wind Direction:  {:.0}°", w.wind_direction);
    println!("Drought Factor:  {:.1}", w.drought_factor);
    println!();
    println!("FFDI:            {:.1}", w.ffdi);
    println!("Fire Danger:     {}", w.fire_danger_rating);
    println!("Spread Mult:     {:.2}x", w.spread_rate_multiplier);
    println!("══════════════════════════════════════════════════\n");
}

fn show_element(sim: &FireSimulation, id: u32) {
    if let Some(e) = sim.get_element(id) {
        let stats = e.get_stats();
        println!("\n═══════════════ ELEMENT {} ═══════════════", id);
        println!(
            "Position:      ({:.1}, {:.1}, {:.1})",
            stats.position.x, stats.position.y, stats.position.z
        );
        println!("Fuel Type:     {}", e.fuel().name);
        println!("Part Type:     {:?}", stats.part_type);
        println!();
        println!("Temperature:   {:.1}°C", stats.temperature);
        println!("Ignition Temp: {:.1}°C", stats.ignition_temperature);
        println!("Ignited:       {}", stats.ignited);
        println!();
        println!("Moisture:      {:.1}%", stats.moisture_fraction * 100.0);
        println!("Fuel Mass:     {:.2} kg", stats.fuel_remaining);
        println!("══════════════════════════════════════════════════\n");
    } else {
        println!("Element {} not found", id);
    }
}

fn show_burning(sim: &FireSimulation) {
    println!("\n═══════════════ BURNING ELEMENTS ═══════════════");
    let burning_elements = sim.get_burning_elements();
    if burning_elements.is_empty() {
        println!("No elements are currently burning.");
    } else {
        println!(
            "{:<6} {:<20} {:<10} {:<10} {:<10}",
            "ID", "Position", "Temp", "Moisture", "Fuel"
        );
        println!("{}", "-".repeat(60));

        for e in burning_elements.iter().take(20) {
            let stats = e.get_stats();
            println!(
                "{:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>8.1}% {:>7.2}kg",
                stats.id,
                stats.position.x,
                stats.position.y,
                stats.position.z,
                stats.temperature,
                stats.moisture_fraction * 100.0,
                stats.fuel_remaining
            );
        }

        if burning_elements.len() > 20 {
            println!("... and {} more", burning_elements.len() - 20);
        }
    }
    println!("══════════════════════════════════════════════════\n");
}

fn show_embers(sim: &FireSimulation) {
    println!("\n═══════════════ ACTIVE EMBERS ═══════════════");
    let ember_count = sim.ember_count();

    if ember_count == 0 {
        println!("No active embers.");
    } else {
        println!("Active embers: {}", ember_count);
    }
    println!("══════════════════════════════════════════════════\n");
}

fn show_nearby(sim: &FireSimulation, id: u32) {
    if let Some(e) = sim.get_element(id) {
        let source_pos = *e.position();
        let nearby = sim.get_elements_in_radius(source_pos, 15.0);

        println!("\n═══════════════ ELEMENTS NEAR {} ═══════════════", id);
        println!(
            "{:<6} {:<20} {:<10} {:<10} {:<8}",
            "ID", "Position", "Temp", "Dist", "Ignited"
        );
        println!("{}", "-".repeat(60));

        for n in nearby.iter().take(15) {
            let stats = n.get_stats();
            if stats.id == id {
                continue;
            }

            let dist = (stats.position - source_pos).magnitude();
            println!(
                "{:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>7.1}m {}",
                stats.id,
                stats.position.x,
                stats.position.y,
                stats.position.z,
                stats.temperature,
                dist,
                if stats.ignited { "🔥" } else { "" }
            );
        }
        println!("══════════════════════════════════════════════════\n");
    } else {
        println!("Element {} not found", id);
    }
}

fn ignite_element(sim: &mut FireSimulation, id: u32) {
    if let Some(e) = sim.get_element(id) {
        let stats = e.get_stats();
        // Start at 600°C - realistic for piloted ignition (matches test values)
        // This represents the rapid flashover when a fuel element catches fire
        // Real fires don't slowly heat from ignition temp - they flash to high temperatures
        let initial_temp = 600.0_f32.max(stats.ignition_temperature);
        sim.ignite_element(id, initial_temp);
        println!(
            "Ignited element {} at ({:.1}, {:.1}, {:.1})",
            id, stats.position.x, stats.position.y, stats.position.z
        );
    } else {
        println!("Element {} not found", id);
    }
}

fn heat_element(sim: &mut FireSimulation, id: u32, target_temp: f32) {
    if let Some(e) = sim.get_element(id) {
        let stats = e.get_stats();
        heat_element_to_temp(sim, id, target_temp);
        println!(
            "Heating element {} to {:.1}°C (was {:.1}°C) at ({:.1}, {:.1}, {:.1})",
            id,
            target_temp,
            stats.temperature,
            stats.position.x,
            stats.position.y,
            stats.position.z
        );
    } else {
        println!("Element {} not found", id);
    }
}

/// Helper function to heat an element to a target temperature
fn heat_element_to_temp(sim: &mut FireSimulation, id: u32, target_temp: f32) {
    if let Some(e) = sim.get_element(id) {
        let stats = e.get_stats();
        let current_temp = stats.temperature;
        let fuel_mass = stats.fuel_remaining;
        let specific_heat = e.fuel().specific_heat;

        if target_temp > current_temp {
            // Calculate heat needed: Q = m × c × ΔT
            let temp_rise = target_temp - current_temp;
            let heat_kj = fuel_mass * specific_heat * temp_rise;

            // Apply heat over 1 second timestep (no pilot flame - external heat source)
            sim.apply_heat_to_element(id, heat_kj, 1.0, false);
        }
    }
}

fn set_preset(sim: &mut FireSimulation, name: &str) {
    let weather = match name.to_lowercase().as_str() {
        "perth" | "perth_metro" => WeatherSystem::from_preset(
            WeatherPreset::perth_metro(),
            3,
            14.0,
            ClimatePattern::Neutral,
        ),
        "catastrophic" | "cat" => WeatherSystem::catastrophic(),
        "goldfields" => WeatherSystem::from_preset(
            WeatherPreset::goldfields(),
            15,
            14.0,
            ClimatePattern::ElNino,
        ),
        "wheatbelt" => {
            WeatherSystem::from_preset(WeatherPreset::wheatbelt(), 15, 14.0, ClimatePattern::ElNino)
        }
        "hot" => {
            let mut w = WeatherSystem::from_preset(
                WeatherPreset::perth_metro(),
                15,
                14.0,
                ClimatePattern::ElNino,
            );

            w.set_temperature(38.0);
            w.set_humidity(20.0);
            w.set_wind_speed(35.0);
            w.set_drought_factor(8.0);

            w
        }
        _ => {
            println!(
                "Unknown preset: {}. Available: perth, catastrophic, goldfields, wheatbelt, hot",
                name
            );
            return;
        }
    };

    sim.set_weather(weather);
    println!("Weather preset changed to '{}'", name);
    show_weather(sim);
}

/// Display an ASCII heatmap of temperature distribution
fn show_heatmap(sim: &FireSimulation, terrain_width: f32, terrain_height: f32, grid_size: usize) {
    println!("\n═══════════════ TEMPERATURE HEATMAP ═══════════════");

    // Create a grid covering the simulation area with appropriate cell size
    let cell_width = terrain_width / grid_size as f32;
    let cell_height = terrain_height / grid_size as f32;

    let mut grid: Vec<Vec<f32>> = vec![vec![0.0; grid_size]; grid_size];
    let mut counts: Vec<Vec<u32>> = vec![vec![0; grid_size]; grid_size];
    let mut burning_grid: Vec<Vec<bool>> = vec![vec![false; grid_size]; grid_size];

    // Accumulate temperatures and track burning state
    for e in sim.get_all_elements() {
        let stats = e.get_stats();
        let x = (stats.position.x / cell_width).floor() as i32;
        let y = (stats.position.y / cell_height).floor() as i32;

        if x >= 0 && x < grid_size as i32 && y >= 0 && y < grid_size as i32 {
            let ix = x as usize;
            let iy = y as usize;
            grid[iy][ix] += stats.temperature;
            counts[iy][ix] += 1;
            if stats.ignited {
                burning_grid[iy][ix] = true;
            }
        }
    }

    // Average temperatures
    for y in 0..grid_size {
        for x in 0..grid_size {
            if counts[y][x] > 0 {
                grid[y][x] /= counts[y][x] as f32;
            }
        }
    }

    // Find temperature range (excluding cells with no elements)
    let mut min_temp = f32::MAX;
    let mut max_temp = f32::MIN;
    for y in 0..grid_size {
        for x in 0..grid_size {
            if counts[y][x] > 0 {
                let temp = grid[y][x];
                min_temp = min_temp.min(temp);
                max_temp = max_temp.max(temp);
            }
        }
    }

    // Handle edge case where no elements exist
    if min_temp == f32::MAX {
        min_temp = 0.0;
        max_temp = 0.0;
    }

    // Minimum absolute temperatures for heat visualization
    // These ensure we don't show "hot" indicators at ambient temperatures
    const MIN_TEMP_COOL: f32 = 50.0;   // Must be above 50°C to show as warming
    const MIN_TEMP_WARM: f32 = 100.0;  // Must be above 100°C to show as warm
    const MIN_TEMP_HOT: f32 = 200.0;   // Must be above 200°C to show as hot
    const MIN_TEMP_VERY_HOT: f32 = 350.0; // Must be above 350°C to show as very hot

    // Calculate dynamic thresholds based on actual temperature range
    // but enforce minimum absolute temperatures
    let temp_range = max_temp - min_temp;
    let threshold_very_hot = (min_temp + temp_range * 0.75).max(MIN_TEMP_VERY_HOT);
    let threshold_hot = (min_temp + temp_range * 0.50).max(MIN_TEMP_HOT);
    let threshold_warm = (min_temp + temp_range * 0.25).max(MIN_TEMP_WARM);
    let threshold_cool = MIN_TEMP_COOL; // Fixed minimum for any heating indication

    // Legend with dynamic values
    println!("Legend: · = empty/ambient  🔥 = burning (ignited)");
    println!("        ░ >{:.0}°C  ▒ >{:.0}°C  ▓ >{:.0}°C  █ >{:.0}°C",
             threshold_cool, threshold_warm, threshold_hot, threshold_very_hot);
    println!("Temperature range: {:.0}°C - {:.0}°C\n", min_temp, max_temp);

    // Print heatmap (top-down view, Y increases downward)
    for y in (0..grid_size).rev() {
        print!("{:3} │ ", (y as f32 * cell_height) as i32);
        for x in 0..grid_size {
            if counts[y][x] == 0 {
                print!("· ");
            } else if burning_grid[y][x] {
                // Actual burning element - use fire emoji or asterisk
                print!("🔥");
            } else {
                let temp = grid[y][x];
                let c = if temp >= threshold_very_hot {
                    '█' // Very hot (>350°C or top 25%)
                } else if temp >= threshold_hot {
                    '▓' // Hot (>200°C or 50-75%)
                } else if temp >= threshold_warm {
                    '▒' // Warm (>100°C or 25-50%)
                } else if temp >= threshold_cool {
                    '░' // Warming (>50°C)
                } else {
                    '·' // At ambient
                };
                print!("{} ", c);
            }
        }
        println!();
    }

    // X-axis labels
    print!("    └");
    for _ in 0..grid_size {
        print!("──");
    }
    println!();
    print!("      ");
    for x in (0..grid_size).step_by(5) {
        print!("{:<10}", (x as f32 * cell_width) as i32);
    }
    println!("\n");

    // Summary stats
    let burning_cells: usize = burning_grid.iter().flatten().filter(|&&b| b).count();
    if burning_cells > 0 {
        println!("Burning cells: {} / {}", burning_cells, grid_size * grid_size);
    }

    println!("══════════════════════════════════════════════════\n");
}

fn show_help() {
    println!("\n═══════════════ AVAILABLE COMMANDS ═══════════════");
    println!("  step [n], s [n]      - Advance n timesteps (default 1)");
    println!("  status, st           - Show simulation status");
    println!("  weather, w           - Show weather conditions");
    println!("  element <id>, e      - Show element details");
    println!("  burning, b           - List burning elements");
    println!("  embers, em           - List active embers");
    println!("  nearby <id>, n       - Show elements near <id>");
    println!("  ignite <id>, i       - Manually ignite element");
    println!("  heat <id> <temp>, h  - Heat element to target temperature (in °C)");
    println!("  ignite_position <x> <y> [radius] [amount] - Ignite elements in an XY circle around (x,y)");
    println!("                         radius defaults to 1.0m; amount: number to ignite (from ground-up). -1 = all (default -1)");
    println!("                         Optional filters: fuel=<name> (substring), part=<partname> (substring), minz=<f32>, maxz=<f32>");
    println!("                         Example: ignite_position 10 20 5 -1 fuel=dry_grass part=groundvegetation minz=0 maxz=0.5");
    println!("  heat_position <x> <y> <temp> [radius] [amount] - Heat elements to target temperature (in °C)");
    println!("                         radius defaults to 1.0m; amount: number to heat (from ground-up). -1 = all (default -1)");
    println!("                         Optional filters: fuel=<name> (substring), part=<partname> (substring), minz=<f32>, maxz=<f32>");
    println!("                         Example: heat_position 10 20 300 5 -1 fuel=dry_grass part=groundvegetation minz=0 maxz=0.5");
    println!("  heatmap, hm [size]   - Show temperature heatmap");
    println!("  preset <name>, p     - Change weather preset");
    println!("                         (perth, catastrophic, goldfields, wheatbelt, hot)");
    println!("  reset [w] [h], r     - Reset simulation (optional: new width/height)");
    println!("  help, ?              - Show this help");
    println!("  quit, q              - Exit");
    println!("══════════════════════════════════════════════════\n");
}
