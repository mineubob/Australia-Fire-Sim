use fire_sim_core::{FireSimulation, Fuel, FuelPart, Vec3, WeatherSystem, WeatherPreset, ClimatePattern};
use clap::Parser;

/// Fire simulation demo with configurable parameters
#[derive(Parser, Debug)]
#[command(name = "fire-sim-demo")]
#[command(about = "Australian wildfire simulation demo", long_about = None)]
struct Args {
    /// Simulation duration in seconds
    #[arg(short, long, default_value_t = 60.0)]
    duration: f32,
    
    /// Temperature in °C
    #[arg(short, long, default_value_t = 30.0)]
    temperature: f32,
    
    /// Relative humidity in %
    #[arg(long, default_value_t = 30.0)]
    humidity: f32,
    
    /// Wind speed in km/h
    #[arg(short, long, default_value_t = 30.0)]
    wind_speed: f32,
    
    /// Wind direction in degrees (0=North, 90=East)
    #[arg(long, default_value_t = 0.0)]
    wind_direction: f32,
    
    /// Drought factor (0-10)
    #[arg(long, default_value_t = 5.0)]
    drought_factor: f32,
    
    /// Use catastrophic conditions preset
    #[arg(short, long)]
    catastrophic: bool,
    
    /// Use regional weather preset (perth-metro, south-west, wheatbelt, goldfields, kimberley, pilbara)
    #[arg(short = 'p', long)]
    preset: Option<String>,
    
    /// Climate pattern (neutral, el-nino, la-nina)
    #[arg(long, default_value = "neutral")]
    climate: String,
    
    /// Day of year (1-365)
    #[arg(long, default_value_t = 15)]
    day: u16,
    
    /// Hour of day (0-24)
    #[arg(long, default_value_t = 14.0)]
    hour: f32,
    
    /// Number of fuel elements (approximate, in hundreds)
    #[arg(short, long, default_value_t = 78)]
    fuel_elements: u32,
    
    /// Report interval in seconds
    #[arg(short, long, default_value_t = 5.0)]
    report_interval: f32,
    
    /// Run validation tests
    #[arg(short, long)]
    validate: bool,
}

fn main() {
    let args = Args::parse();
    
    println!("=== Fire Simulation Demo ===\n");
    
    // Create simulation
    let mut sim = FireSimulation::new(1000.0, 1000.0, 100.0);
    println!("Created simulation with 1000x1000x100m bounds");
    
    // Parse climate pattern
    let climate_pattern = match args.climate.to_lowercase().as_str() {
        "el-nino" | "elnino" => ClimatePattern::ElNino,
        "la-nina" | "lanina" => ClimatePattern::LaNina,
        _ => ClimatePattern::Neutral,
    };
    
    // Set weather based on arguments or preset
    let weather = if args.catastrophic {
        println!("Using CATASTROPHIC weather preset");
        WeatherSystem::catastrophic()
    } else if let Some(preset_name) = &args.preset {
        // Use regional preset
        let preset = match preset_name.to_lowercase().as_str() {
            "perth-metro" | "perth" => WeatherPreset::perth_metro(),
            "south-west" | "southwest" => WeatherPreset::south_west(),
            "wheatbelt" => WeatherPreset::wheatbelt(),
            "goldfields" => WeatherPreset::goldfields(),
            "kimberley" => WeatherPreset::kimberley(),
            "pilbara" => WeatherPreset::pilbara(),
            _ => {
                println!("Unknown preset '{}', using Perth Metro", preset_name);
                WeatherPreset::perth_metro()
            }
        };
        println!("Using '{}' regional preset", preset.name);
        println!("Climate: {:?}, Day: {}, Hour: {:.1}", climate_pattern, args.day, args.hour);
        WeatherSystem::from_preset(preset, args.day, args.hour, climate_pattern)
    } else {
        WeatherSystem::new(
            args.temperature,
            args.humidity,
            args.wind_speed,
            args.wind_direction,
            args.drought_factor,
        )
    };
    
    println!("Weather: {} (FFDI: {:.1})", weather.fire_danger_rating(), weather.calculate_ffdi());
    println!("Wind: {:.1} km/h, Temp: {:.1}°C, Humidity: {:.1}%, Drought: {:.1}", 
             weather.wind_speed, weather.temperature, weather.humidity, weather.drought_factor);
    if let Some(preset_name) = weather.preset_name() {
        println!("Region: {}, Solar: {:.0} W/m², Curing: {:.0}%\n", 
                 preset_name, weather.solar_radiation(), weather.fuel_curing());
    } else {
        println!();
    }
    sim.set_weather(weather);
    
    // Create a test scenario: grass field with eucalyptus trees
    println!("Creating fuel elements...");
    
    // Scale the number of elements based on argument
    let radius_squared = ((args.fuel_elements as f32) * 32.0) as i32;
    
    // Add ground cover (dry grass)
    let grass_fuel = Fuel::dry_grass();
    for x in -50..=50 {
        for y in -50..=50 {
            if (x * x + y * y) < radius_squared { // Circular area
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
    let mut next_report = 0.0;
    
    while time < args.duration && sim.burning_count() > 0 {
        sim.update(dt);
        time += dt;
        
        if time >= next_report {
            println!("{:7.1} | {:7} | {:6} | {:17.2}",
                     time,
                     sim.burning_count(),
                     sim.ember_count(),
                     sim.total_fuel_consumed);
            next_report += args.report_interval;
        }
    }
    
    println!("\n=== Simulation Complete ===");
    println!("Final time: {:.1}s", time);
    println!("Total fuel consumed: {:.2} kg", sim.total_fuel_consumed);
    println!("Peak burning elements: {}", sim.burning_count());
    
    // Run validation tests if requested
    if args.validate {
        run_validation_tests();
    }
}

fn run_validation_tests() {
    println!("\n=== Running Validation Tests ===\n");    
    // Test 1: Wind directionality
    println!("Test 1: Wind Directionality");
    let mut sim = FireSimulation::new(100.0, 100.0, 50.0);
    let weather = WeatherSystem::new(30.0, 30.0, 40.0, 0.0, 5.0);
    sim.set_weather(weather);
    
    let fuel = Fuel::dry_grass();
    let source = sim.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    let downwind = sim.add_fuel_element(Vec3::new(10.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    let upwind = sim.add_fuel_element(Vec3::new(-10.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
    
    sim.ignite_element(source, 800.0);
    
    for _ in 0..50 {
        sim.update(0.1);
    }
    
    let down_temp = sim.get_element(downwind).map(|e| e.temperature).unwrap_or(0.0);
    let up_temp = sim.get_element(upwind).map(|e| e.temperature).unwrap_or(0.0);
    let ratio = if up_temp > 25.0 { down_temp / up_temp } else { 1.0 };
    
    println!("  Downwind element temp: {:.1}°C", down_temp);
    println!("  Upwind element temp: {:.1}°C", up_temp);
    println!("  Ratio: {:.1}x", ratio);
    if ratio > 2.0 {
        println!("  ✓ PASS: Fire spreads faster downwind");
    } else {
        println!("  ✗ FAIL: Expected stronger directional spread");
    }
    
    // Test 2: Moisture evaporation
    println!("\nTest 2: Moisture Evaporation Delay");
    let mut sim2 = FireSimulation::new(100.0, 100.0, 50.0);
    let weather2 = WeatherSystem::new(30.0, 30.0, 20.0, 0.0, 5.0);
    sim2.set_weather(weather2);
    
    let dry_fuel = Fuel::dead_wood_litter();
    let mut wet_fuel = Fuel::green_vegetation();
    wet_fuel.base_moisture = 0.20;
    
    let dry = sim2.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), dry_fuel, 1.0, FuelPart::GroundLitter, None);
    let wet = sim2.add_fuel_element(Vec3::new(5.0, 0.0, 0.0), wet_fuel, 1.0, FuelPart::GroundVegetation, None);
    
    sim2.ignite_element(dry, 600.0);
    sim2.ignite_element(wet, 600.0);
    
    for _ in 0..20 {
        sim2.update(0.1);
    }
    
    let dry_temp = sim2.get_element(dry).map(|e| e.temperature).unwrap_or(0.0);
    let wet_temp = sim2.get_element(wet).map(|e| e.temperature).unwrap_or(0.0);
    
    println!("  Dry fuel (5% moisture) temp: {:.1}°C", dry_temp);
    println!("  Wet fuel (20% moisture) temp: {:.1}°C", wet_temp);
    if dry_temp > wet_temp + 50.0 {
        println!("  ✓ PASS: Moisture slows heating");
    } else {
        println!("  ✗ FAIL: Expected stronger moisture effect");
    }
    
    // Test 3: Vertical spread
    println!("\nTest 3: Vertical Fire Spread");
    let mut sim3 = FireSimulation::new(100.0, 100.0, 50.0);
    let weather3 = WeatherSystem::new(30.0, 30.0, 20.0, 0.0, 5.0);
    sim3.set_weather(weather3);
    
    let tree_fuel = Fuel::eucalyptus_stringybark();
    let lower = sim3.add_fuel_element(Vec3::new(0.0, 0.0, 0.0), tree_fuel.clone(), 5.0, FuelPart::TrunkLower, None);
    let upper = sim3.add_fuel_element(Vec3::new(0.0, 0.0, 10.0), tree_fuel.clone(), 5.0, FuelPart::TrunkUpper, Some(lower));
    
    sim3.ignite_element(lower, 800.0);
    
    for _ in 0..100 {
        sim3.update(0.1);
    }
    
    let upper_temp = sim3.get_element(upper).map(|e| e.temperature).unwrap_or(0.0);
    
    println!("  Upper element temp: {:.1}°C", upper_temp);
    if upper_temp > 300.0 {
        println!("  ✓ PASS: Fire climbed to upper element");
    } else {
        println!("  ✗ FAIL: Fire did not climb");
    }
    
    println!("\n=== Validation Complete ===");
}
