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
//! - `preset <name>` - Switch weather preset (perth, catastrophic, etc.)
//! - `help` - Show available commands
//! - `quit` - Exit the simulation

use fire_sim_core::{
    ClimatePattern, FireSimulation, Fuel, FuelElement, FuelPart, Vec3, WeatherPreset, WeatherSystem,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      Australia Fire Simulation - Interactive Debugger     ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Create simulation with default setup
    let mut sim = create_test_simulation();
    println!("Created simulation with {} elements", sim.element_count());

    // Start with one ignited element
    if let Some(id) = sim.elements().flatten().next().map(|e| e.id) {
        ignite_element(&mut sim, id);
    }

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
                let parts: Vec<&str> = line.trim().split_whitespace().collect();

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
                    "preset" | "p" => {
                        if let Some(name) = parts.get(1) {
                            set_preset(&mut sim, name);
                        } else {
                            println!("Usage: preset <perth|catastrophic|goldfields|wheatbelt>");
                        }
                    }
                    "heat" | "h" => {
                        if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                            let amount = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(100.0);
                            add_heat(&mut sim, id, amount);
                        } else {
                            println!("Usage: heat <id> [amount]");
                        }
                    }
                    "help" | "?" => show_help(),
                    "quit" | "q" | "exit" => {
                        println!("Goodbye!");
                        break;
                    }
                    _ => println!("Unknown command: {}. Type 'help' for available commands.", parts[0]),
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

fn create_test_simulation() -> FireSimulation {
    let mut sim = FireSimulation::new(150.0, 150.0, 30.0);

    // Create a grid of fuel elements representing different vegetation
    // Ground layer: grass and shrubs
    for x in (0..150).step_by(5) {
        for y in (0..150).step_by(5) {
            let fuel = if (x + y) % 20 == 0 {
                Fuel::shrubland_scrub()
            } else {
                Fuel::dry_grass()
            };
            
            let id = sim.add_element(
                Vec3::new(x as f32, y as f32, 0.0),
                fuel,
                3.0,
                FuelPart::GroundVegetation,
                None,
            );

            // Add some trees
            if x % 15 == 0 && y % 15 == 0 {
                create_tree(&mut sim, x as f32, y as f32, id);
            }
        }
    }

    // Set to Perth Metro conditions
    let weather = WeatherSystem::from_preset(
        WeatherPreset::perth_metro(),
        3, // January 3
        14.0, // 2pm
        ClimatePattern::Neutral,
    );
    sim.set_weather(weather);

    sim
}

fn create_tree(sim: &mut FireSimulation, x: f32, y: f32, _ground_id: u32) {
    // Trunk
    let trunk_id = sim.add_element(
        Vec3::new(x, y, 2.0),
        Fuel::eucalyptus_stringybark(),
        10.0,
        FuelPart::TrunkLower,
        None,
    );

    // Lower branches
    sim.add_element(
        Vec3::new(x - 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        3.0,
        FuelPart::BranchLower,
        Some(trunk_id),
    );
    sim.add_element(
        Vec3::new(x + 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        3.0,
        FuelPart::BranchLower,
        Some(trunk_id),
    );

    // Crown
    sim.add_element(
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
        let burning_before = sim.burning_elements.len();
        let embers_before = sim.embers().count();

        sim.update(dt);

        let burning_after = sim.burning_elements.len();
        let embers_after = sim.embers().count();

        if i == count - 1 || burning_after != burning_before || embers_after != embers_before {
            println!(
                "  Step {}: Burning: {} → {}, Embers: {} → {}",
                i + 1,
                burning_before,
                burning_after,
                embers_before,
                embers_after
            );
        }
    }
    println!("Done.");
}

fn show_status(sim: &FireSimulation) {
    println!("\n═══════════════ SIMULATION STATUS ═══════════════");
    println!("Total elements:    {}", sim.element_count());
    println!("Burning elements:  {}", sim.burning_elements.len());
    println!("Active embers:     {}", sim.embers().count());
    
    // Find temperature range of burning elements
    let burning: Vec<_> = sim.burning_elements.iter().filter_map(|&id| sim.get_element(id)).collect();
    if !burning.is_empty() {
        let min_temp = burning.iter().map(|e| e.temperature).fold(f32::MAX, f32::min);
        let max_temp = burning.iter().map(|e| e.temperature).fold(f32::MIN, f32::max);
        let avg_temp: f32 = burning.iter().map(|e| e.temperature).sum::<f32>() / burning.len() as f32;
        
        println!("\nBurning element temperatures:");
        println!("  Min: {:.1}°C", min_temp);
        println!("  Max: {:.1}°C", max_temp);
        println!("  Avg: {:.1}°C", avg_temp);
    }
    
    println!("══════════════════════════════════════════════════\n");
}

fn show_weather(sim: &FireSimulation) {
    let w = &sim.weather;
    println!("\n═══════════════ WEATHER CONDITIONS ═══════════════");
    println!("Temperature:     {:.1}°C", w.temperature);
    println!("Humidity:        {:.1}%", w.humidity);
    println!("Wind Speed:      {:.1} km/h ({:.1} m/s)", w.wind_speed, w.wind_speed / 3.6);
    println!("Wind Direction:  {:.0}°", w.wind_direction);
    println!("Drought Factor:  {:.1}", w.drought_factor);
    println!();
    println!("FFDI:            {:.1}", w.calculate_ffdi());
    println!("Fire Danger:     {}", w.fire_danger_rating());
    println!("Spread Mult:     {:.2}x", w.spread_rate_multiplier());
    println!("══════════════════════════════════════════════════\n");
}

fn show_element(sim: &FireSimulation, id: u32) {
    if let Some(e) = sim.get_element(id) {
        println!("\n═══════════════ ELEMENT {} ═══════════════", id);
        println!("Position:      ({:.1}, {:.1}, {:.1})", e.position.x, e.position.y, e.position.z);
        println!("Fuel Type:     {:?}", e.fuel.fuel_type_id);
        println!("Part Type:     {:?}", e.part_type);
        println!();
        println!("Temperature:   {:.1}°C", e.temperature);
        println!("Ignition Temp: {:.1}°C", e.fuel.ignition_temperature);
        println!("Ignited:       {}", e.ignited);
        println!();
        println!("Moisture:      {:.1}%", e.moisture_fraction * 100.0);
        println!("Fuel Mass:     {:.2} kg", e.fuel_remaining);
        println!("══════════════════════════════════════════════════\n");
    } else {
        println!("Element {} not found", id);
    }
}

fn show_burning(sim: &FireSimulation) {
    println!("\n═══════════════ BURNING ELEMENTS ═══════════════");
    if sim.burning_elements.is_empty() {
        println!("No elements are currently burning.");
    } else {
        println!("{:<6} {:<20} {:<10} {:<10} {:<10}", "ID", "Position", "Temp", "Moisture", "Fuel");
        println!("{}", "-".repeat(60));
        
        for &id in sim.burning_elements.iter().take(20) {
            if let Some(e) = sim.get_element(id) {
                println!(
                    "{:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>8.1}% {:>7.2}kg",
                    id,
                    e.position.x,
                    e.position.y,
                    e.position.z,
                    e.temperature,
                    e.moisture_fraction * 100.0,
                    e.fuel_remaining
                );
            }
        }
        
        if sim.burning_elements.len() > 20 {
            println!("... and {} more", sim.burning_elements.len() - 20);
        }
    }
    println!("══════════════════════════════════════════════════\n");
}

fn show_embers(sim: &FireSimulation) {
    println!("\n═══════════════ ACTIVE EMBERS ═══════════════");
    let embers: Vec<_> = sim.embers().collect();
    
    if embers.is_empty() {
        println!("No active embers.");
    } else {
        println!("{:<6} {:<25} {:<10} {:<8}", "ID", "Position", "Temp", "Mass");
        println!("{}", "-".repeat(55));
        
        for e in embers.iter().take(10) {
            println!(
                "{:<6} ({:>6.1}, {:>6.1}, {:>6.1}) {:>7.1}°C {:>6.2}g",
                e.id,
                e.position.x,
                e.position.y,
                e.position.z,
                e.temperature,
                e.mass * 1000.0 // Convert to grams
            );
        }
        
        if embers.len() > 10 {
            println!("... and {} more", embers.len() - 10);
        }
    }
    println!("══════════════════════════════════════════════════\n");
}

fn show_nearby(sim: &FireSimulation, id: u32) {
    if let Some(e) = sim.get_element(id) {
        let nearby = sim.spatial_index.query_radius(e.position, 15.0);
        
        println!("\n═══════════════ ELEMENTS NEAR {} ═══════════════", id);
        println!("{:<6} {:<20} {:<10} {:<10} {:<8}", "ID", "Position", "Temp", "Dist", "Ignited");
        println!("{}", "-".repeat(60));
        
        for &nearby_id in nearby.iter().take(15) {
            if nearby_id == id {
                continue;
            }
            if let Some(n) = sim.get_element(nearby_id) {
                let dist = (n.position - e.position).magnitude();
                println!(
                    "{:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>7.1}m {}",
                    nearby_id,
                    n.position.x,
                    n.position.y,
                    n.position.z,
                    n.temperature,
                    dist,
                    if n.ignited { "🔥" } else { "" }
                );
            }
        }
        println!("══════════════════════════════════════════════════\n");
    } else {
        println!("Element {} not found", id);
    }
}

fn ignite_element(sim: &mut FireSimulation, id: u32) {
    if let Some(e) = sim.get_element_mut(id) {
        e.temperature = e.fuel.ignition_temperature + 100.0;
        e.ignited = true;
        sim.burning_elements.insert(id);
        println!("Ignited element {} at ({:.1}, {:.1}, {:.1})", 
            id, e.position.x, e.position.y, e.position.z);
    } else {
        println!("Element {} not found", id);
    }
}

fn add_heat(sim: &mut FireSimulation, id: u32, amount: f32) {
    let ambient_temp = sim.grid.ambient_temperature;
    if let Some(e) = sim.get_element_mut(id) {
        e.apply_heat(amount, 1.0, ambient_temp);
        println!("Added {:.1} kJ to element {}. New temperature: {:.1}°C", 
            amount, id, e.temperature);
        
        if e.temperature > e.fuel.ignition_temperature && !e.ignited {
            e.ignited = true;
            sim.burning_elements.insert(id);
            println!("Element {} has ignited!", id);
        }
    } else {
        println!("Element {} not found", id);
    }
}

fn set_preset(sim: &mut FireSimulation, name: &str) {
    let weather = match name.to_lowercase().as_str() {
        "perth" | "perth_metro" => {
            WeatherSystem::from_preset(
                WeatherPreset::perth_metro(),
                3, 14.0,
                ClimatePattern::Neutral,
            )
        }
        "catastrophic" | "cat" => WeatherSystem::catastrophic(),
        "goldfields" => {
            WeatherSystem::from_preset(
                WeatherPreset::goldfields(),
                15, 14.0,
                ClimatePattern::ElNino,
            )
        }
        "wheatbelt" => {
            WeatherSystem::from_preset(
                WeatherPreset::wheatbelt(),
                15, 14.0,
                ClimatePattern::ElNino,
            )
        }
        "hot" => {
            let mut w = WeatherSystem::from_preset(
                WeatherPreset::perth_metro(),
                15, 14.0,
                ClimatePattern::ElNino,
            );
            w.temperature = 38.0;
            w.humidity = 20.0;
            w.wind_speed = 35.0;
            w.drought_factor = 8.0;
            w
        }
        _ => {
            println!("Unknown preset: {}. Available: perth, catastrophic, goldfields, wheatbelt, hot", name);
            return;
        }
    };
    
    sim.set_weather(weather);
    println!("Weather preset changed to '{}'", name);
    show_weather(sim);
}

fn show_help() {
    println!("\n═══════════════ AVAILABLE COMMANDS ═══════════════");
    println!("  step [n], s [n]   - Advance n timesteps (default 1)");
    println!("  status, st        - Show simulation status");
    println!("  weather, w        - Show weather conditions");
    println!("  element <id>, e   - Show element details");
    println!("  burning, b        - List burning elements");
    println!("  embers, em        - List active embers");
    println!("  nearby <id>, n    - Show elements near <id>");
    println!("  ignite <id>, i    - Manually ignite element");
    println!("  heat <id> [amt]   - Add heat to element");
    println!("  preset <name>, p  - Change weather preset");
    println!("                      (perth, catastrophic, goldfields, wheatbelt, hot)");
    println!("  help, ?           - Show this help");
    println!("  quit, q           - Exit");
    println!("══════════════════════════════════════════════════\n");
}
