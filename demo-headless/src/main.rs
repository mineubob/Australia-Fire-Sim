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
    
    // Test specific scenarios
    println!("\n=== Running Validation Tests ===\n");
    run_validation_tests();
}

fn run_validation_tests() {
    // Test 1: Wind directionality
    println!("Test 1: Wind Directionality");
    let mut sim = FireSimulation::new(200.0, 200.0, 50.0);
    
    // Set strong wind
    let weather = WeatherSystem::new(30.0, 20.0, 50.0, 0.0, 7.0);
    sim.set_weather(weather);
    
    let fuel = Fuel::dry_grass();
    
    // Create line of fuel perpendicular to wind
    let source = sim.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    let downwind = sim.add_fuel_element(Vec3::new(5.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    let upwind = sim.add_fuel_element(Vec3::new(-5.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    
    sim.ignite_element(source, 600.0);
    
    // Run for a bit
    for _ in 0..100 {
        sim.update(0.1);
    }
    
    let downwind_temp = sim.get_element(downwind).unwrap().temperature;
    let upwind_temp = sim.get_element(upwind).unwrap().temperature;
    
    println!("  Downwind element temp: {:.1}°C", downwind_temp);
    println!("  Upwind element temp: {:.1}°C", upwind_temp);
    println!("  Ratio: {:.1}x", downwind_temp / upwind_temp.max(20.0));
    
    if downwind_temp > upwind_temp * 2.0 {
        println!("  ✓ PASS: Fire spreads faster downwind\n");
    } else {
        println!("  ✗ FAIL: Expected stronger directional spread\n");
    }
    
    // Test 2: Moisture evaporation delay
    println!("Test 2: Moisture Evaporation Delay");
    let mut sim_dry = FireSimulation::new(100.0, 100.0, 50.0);
    let mut sim_wet = FireSimulation::new(100.0, 100.0, 50.0);
    
    sim_dry.set_weather(WeatherSystem::default());
    sim_wet.set_weather(WeatherSystem::default());
    
    let mut fuel_dry = Fuel::dry_grass();
    fuel_dry.base_moisture = 0.05;
    
    let mut fuel_wet = Fuel::dry_grass();
    fuel_wet.base_moisture = 0.20;
    
    let dry_id = sim_dry.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), fuel_dry, 1.0, FuelPart::GroundVegetation, None);
    let wet_id = sim_wet.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), fuel_wet, 1.0, FuelPart::GroundVegetation, None);
    
    // Apply same heat to both
    for _ in 0..50 {
        if let Some(element) = sim_dry.get_element_mut(dry_id) {
            element.apply_heat(100.0, 0.1, 20.0);
        }
        if let Some(element) = sim_wet.get_element_mut(wet_id) {
            element.apply_heat(100.0, 0.1, 20.0);
        }
    }
    
    let dry_temp = sim_dry.get_element(dry_id).unwrap().temperature;
    let wet_temp = sim_wet.get_element(wet_id).unwrap().temperature;
    
    println!("  Dry fuel (5% moisture) temp: {:.1}°C", dry_temp);
    println!("  Wet fuel (20% moisture) temp: {:.1}°C", wet_temp);
    
    if dry_temp > wet_temp * 1.5 {
        println!("  ✓ PASS: Moisture delays heating\n");
    } else {
        println!("  ✗ FAIL: Expected stronger moisture effect\n");
    }
    
    // Test 3: Vertical spread
    println!("Test 3: Vertical Fire Spread");
    let mut sim = FireSimulation::new(100.0, 100.0, 50.0);
    sim.set_weather(WeatherSystem::default());
    
    let fuel = Fuel::eucalyptus_stringybark();
    let lower = sim.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), fuel.clone(), 2.0, FuelPart::TrunkLower, None);
    let upper = sim.add_fuel_element(Vec3::new(0.0, 0.0, 5.0), fuel.clone(), 2.0, FuelPart::TrunkUpper, None);
    
    sim.ignite_element(lower, 700.0);
    
    // Run simulation
    for _ in 0..200 {
        sim.update(0.1);
        if sim.get_element(upper).unwrap().ignited {
            break;
        }
    }
    
    if sim.get_element(upper).unwrap().ignited {
        println!("  ✓ PASS: Fire climbed to upper element\n");
    } else {
        println!("  ✗ FAIL: Fire did not climb (may need more time or closer spacing)\n");
    }
    
    println!("=== Validation Complete ===");
}
