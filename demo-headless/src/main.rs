use fire_sim_core::{FireSimulation, Fuel, FuelPart, Vec3, WeatherSystem};

fn main() {
    println!("=== Fire Simulation Demo ===\n");
    
    // Create simulation
    let mut sim = FireSimulation::new(1000.0, 1000.0, 100.0);
    println!("Created simulation with 1000x1000x100m bounds");
    
    // Set weather to extreme conditions
    let weather = WeatherSystem::catastrophic();
    println!("Weather: {} (FFDI: {:.1})", weather.fire_danger_rating(), weather.calculate_ffdi());
    println!("Wind: {:.1} km/h, Temp: {:.1}°C, Humidity: {:.1}%\n", 
             weather.wind_speed, weather.temperature, weather.humidity);
    sim.set_weather(weather);
    
    // Create a test scenario: grass field with eucalyptus trees
    println!("Creating fuel elements...");
    
    // Add ground cover (dry grass)
    let grass_fuel = Fuel::dry_grass();
    for x in -50..=50 {
        for y in -50..=50 {
            if (x * x + y * y) < 2500 { // Circular area
                sim.add_fuel_element(
                    Vec3::new(x as f32 * 2.0, y as f32 * 2.0, 0.0),
                    grass_fuel.clone(),
                    0.5,
                    FuelPart::GroundVegetation,
                    None,
                );
            }
        }
    }
    
    // Add a few eucalyptus stringybark trees
    let stringybark = Fuel::eucalyptus_stringybark();
    for i in 0..5 {
        let x = (i as f32 - 2.0) * 20.0;
        let tree_base = sim.add_fuel_element(
            Vec3::new(x, 0.0, 0.0),
            stringybark.clone(),
            10.0,
            FuelPart::TrunkLower,
            None,
        );
        
        // Add trunk middle
        sim.add_fuel_element(
            Vec3::new(x, 0.0, 5.0),
            stringybark.clone(),
            8.0,
            FuelPart::TrunkMiddle,
            Some(tree_base),
        );
        
        // Add trunk upper
        sim.add_fuel_element(
            Vec3::new(x, 0.0, 10.0),
            stringybark.clone(),
            6.0,
            FuelPart::TrunkUpper,
            Some(tree_base),
        );
        
        // Add crown
        sim.add_fuel_element(
            Vec3::new(x, 0.0, 15.0),
            stringybark.clone(),
            5.0,
            FuelPart::Crown,
            Some(tree_base),
        );
    }
    
    println!("Created {} fuel elements", sim.element_count());
    
    // Ignite the center
    println!("\nIgniting fire at origin...\n");
    let center_elements: Vec<u32> = (0..sim.element_count() as u32).collect();
    for id in center_elements {
        if let Some(element) = sim.get_element(id) {
            if element.position.magnitude() < 5.0 {
                sim.ignite_element(id, 600.0);
            }
        }
    }
    
    // Run simulation
    println!("Running simulation...\n");
    println!("Time(s) | Burning | Embers | Fuel Consumed(kg)");
    println!("--------|---------|--------|------------------");
    
    let mut time = 0.0;
    let dt = 0.1; // 10 Hz update rate
    let report_interval = 5.0; // Report every 5 seconds
    let mut next_report = 0.0;
    
    while time < 60.0 && sim.burning_count() > 0 {
        sim.update(dt);
        time += dt;
        
        if time >= next_report {
            println!("{:7.1} | {:7} | {:6} | {:17.2}",
                     time,
                     sim.burning_count(),
                     sim.ember_count(),
                     sim.total_fuel_consumed);
            next_report += report_interval;
        }
    }
    
    println!("\n=== Simulation Complete ===");
    println!("Final time: {:.1}s", time);
    println!("Total fuel consumed: {:.2} kg", sim.total_fuel_consumed);
    println!("Peak burning elements: {}", sim.burning_count());
}