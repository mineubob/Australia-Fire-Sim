use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use fire_sim_core::{FireSimulation, Fuel, FuelPart, Vec3 as SimVec3, WeatherSystem, ClimatePattern, WeatherPreset};

// Constants
const SCALE_FACTOR: f32 = 0.1; // Scale simulation coordinates to Bevy world
const FIRE_COLOR_LOW: Color = Color::rgb(1.0, 0.8, 0.0);
const FIRE_COLOR_MEDIUM: Color = Color::rgb(1.0, 0.5, 0.0);
const FIRE_COLOR_HIGH: Color = Color::rgb(1.0, 0.2, 0.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Australia Fire Simulation - Bevy Demo".to_string(),
                resolution: (1600., 900.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .init_resource::<SimulationState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            ui_system,
            update_simulation,
            update_fire_visualization,
            update_ember_visualization,
            camera_controls,
        ))
        .run();
}

#[derive(Resource)]
struct SimulationState {
    simulation: FireSimulation,
    paused: bool,
    time: f32,
    dt: f32,
    
    // UI controls
    temperature: f32,
    humidity: f32,
    wind_speed: f32,
    wind_direction: f32,
    drought_factor: f32,
    
    // Spawn controls
    spawn_fuel_type: FuelType,
    
    // Statistics
    total_elements: usize,
    burning_elements: usize,
    ember_count: usize,
    fuel_consumed: f32,
    ffdi: f32,
    fire_danger_rating: String,
}

#[derive(Clone, Copy, PartialEq)]
enum FuelType {
    DryGrass,
    StringyBark,
    SmoothBark,
    Shrubland,
    DeadWood,
}

impl Default for SimulationState {
    fn default() -> Self {
        let mut sim = FireSimulation::new(500.0, 500.0, 100.0);
        let weather = WeatherSystem::new(30.0, 30.0, 30.0, 0.0, 5.0);
        sim.set_weather(weather.clone());
        
        Self {
            simulation: sim,
            paused: false,
            time: 0.0,
            dt: 0.1,
            temperature: 30.0,
            humidity: 30.0,
            wind_speed: 30.0,
            wind_direction: 0.0,
            drought_factor: 5.0,
            spawn_fuel_type: FuelType::DryGrass,
            total_elements: 0,
            burning_elements: 0,
            ember_count: 0,
            fuel_consumed: 0.0,
            ffdi: weather.calculate_ffdi(),
            fire_danger_rating: weather.fire_danger_rating().to_string(),
        }
    }
}

// Component to mark fire element sprites
#[derive(Component)]
#[allow(dead_code)]
struct FireElement {
    element_id: u32,
}

// Component to mark ember sprites
#[derive(Component)]
#[allow(dead_code)]
struct EmberSprite {
    ember_index: usize,
}

fn setup(mut commands: Commands) {
    // Setup 2D camera
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1000.0),
        ..default()
    });
    
    println!("=== Australia Fire Simulation - Bevy Demo ===");
    println!("Controls:");
    println!("  Left Click: Add fuel element");
    println!("  Right Click: Ignite element at cursor");
    println!("  Arrow Keys: Pan camera");
    println!("  +/-: Zoom in/out");
    println!("  Space: Pause/Resume");
    println!("  R: Reset simulation");
    println!();
    println!("Use the UI panel on the right to control weather and simulation parameters.");
}

fn camera_controls(
    keyboard: Res<Input<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    let mut camera_transform = camera_query.single_mut();
    let speed = 500.0 * time.delta_seconds();
    let zoom_speed = 2.0;
    
    // Pan
    if keyboard.pressed(KeyCode::Left) {
        camera_transform.translation.x -= speed;
    }
    if keyboard.pressed(KeyCode::Right) {
        camera_transform.translation.x += speed;
    }
    if keyboard.pressed(KeyCode::Up) {
        camera_transform.translation.y += speed;
    }
    if keyboard.pressed(KeyCode::Down) {
        camera_transform.translation.y -= speed;
    }
    
    // Zoom
    if keyboard.pressed(KeyCode::Equals) || keyboard.pressed(KeyCode::NumpadAdd) {
        camera_transform.scale *= 1.0 - zoom_speed * time.delta_seconds();
        camera_transform.scale.x = camera_transform.scale.x.max(0.1);
        camera_transform.scale.y = camera_transform.scale.y.max(0.1);
    }
    if keyboard.pressed(KeyCode::Minus) || keyboard.pressed(KeyCode::NumpadSubtract) {
        camera_transform.scale *= 1.0 + zoom_speed * time.delta_seconds();
        camera_transform.scale.x = camera_transform.scale.x.min(5.0);
        camera_transform.scale.y = camera_transform.scale.y.min(5.0);
    }
}

fn update_simulation(
    mut state: ResMut<SimulationState>,
    keyboard: Res<Input<KeyCode>>,
    _time: Res<Time>,
) {
    // Toggle pause
    if keyboard.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
        println!("Simulation {}", if state.paused { "paused" } else { "resumed" });
    }
    
    // Reset
    if keyboard.just_pressed(KeyCode::R) {
        println!("Resetting simulation...");
        let mut new_sim = FireSimulation::new(500.0, 500.0, 100.0);
        let weather = WeatherSystem::new(
            state.temperature,
            state.humidity,
            state.wind_speed,
            state.wind_direction,
            state.drought_factor,
        );
        new_sim.set_weather(weather);
        state.simulation = new_sim;
        state.time = 0.0;
    }
    
    // Update simulation
    if !state.paused {
        let dt = state.dt;
        state.simulation.update(dt);
        state.time += dt;
    }
    
    // Update statistics
    state.total_elements = state.simulation.element_count();
    state.burning_elements = state.simulation.burning_count();
    state.ember_count = state.simulation.ember_count();
    state.fuel_consumed = state.simulation.total_fuel_consumed;
}

fn ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<SimulationState>,
    mouse_button: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let ctx = contexts.ctx_mut();
    
    egui::SidePanel::right("control_panel")
        .default_width(350.0)
        .show(ctx, |ui| {
            ui.heading("🔥 Fire Simulation Control");
            ui.separator();
            
            // Status
            ui.label(format!("Time: {:.1}s", state.time));
            ui.label(format!("Status: {}", if state.paused { "⏸ Paused" } else { "▶ Running" }));
            ui.separator();
            
            // Statistics
            ui.heading("📊 Statistics");
            ui.label(format!("Total Elements: {}", state.total_elements));
            ui.label(format!("🔥 Burning: {}", state.burning_elements));
            ui.label(format!("✨ Embers: {}", state.ember_count));
            ui.label(format!("🪵 Fuel Consumed: {:.1} kg", state.fuel_consumed));
            ui.separator();
            
            // Fire Danger
            ui.heading("⚠️ Fire Danger");
            ui.label(format!("FFDI: {:.1}", state.ffdi));
            
            // Color-code the fire danger rating
            let danger_color = match state.fire_danger_rating.as_str() {
                "CATASTROPHIC" => egui::Color32::from_rgb(139, 0, 0),
                "Extreme" => egui::Color32::from_rgb(255, 0, 0),
                "Severe" => egui::Color32::from_rgb(255, 69, 0),
                "Very High" => egui::Color32::from_rgb(255, 140, 0),
                "High" => egui::Color32::from_rgb(255, 215, 0),
                "Moderate" => egui::Color32::from_rgb(50, 205, 50),
                _ => egui::Color32::from_rgb(135, 206, 235),
            };
            
            ui.colored_label(danger_color, format!("Rating: {}", state.fire_danger_rating));
            ui.separator();
            
            // Weather Controls
            ui.heading("🌤️ Weather");
            
            ui.label("Temperature (°C):");
            if ui.add(egui::Slider::new(&mut state.temperature, 10.0..=50.0)).changed() {
                update_weather(&mut state);
            }
            
            ui.label("Humidity (%):");
            if ui.add(egui::Slider::new(&mut state.humidity, 10.0..=90.0)).changed() {
                update_weather(&mut state);
            }
            
            ui.label("Wind Speed (km/h):");
            if ui.add(egui::Slider::new(&mut state.wind_speed, 0.0..=100.0)).changed() {
                update_weather(&mut state);
            }
            
            ui.label("Wind Direction (°):");
            if ui.add(egui::Slider::new(&mut state.wind_direction, 0.0..=360.0)).changed() {
                update_weather(&mut state);
            }
            
            ui.label("Drought Factor (0-10):");
            if ui.add(egui::Slider::new(&mut state.drought_factor, 0.0..=10.0)).changed() {
                update_weather(&mut state);
            }
            
            ui.separator();
            
            // Weather Presets
            ui.heading("📍 Quick Presets");
            if ui.button("🔥 Catastrophic").clicked() {
                let weather = WeatherSystem::catastrophic();
                state.temperature = weather.temperature;
                state.humidity = weather.humidity;
                state.wind_speed = weather.wind_speed;
                state.wind_direction = weather.wind_direction;
                state.drought_factor = weather.drought_factor;
                update_weather(&mut state);
                println!("Applied CATASTROPHIC weather preset");
            }
            
            if ui.button("☀️ Perth Summer").clicked() {
                let weather = WeatherSystem::from_preset(
                    WeatherPreset::perth_metro(),
                    15, // January 15
                    14.0, // 2pm
                    ClimatePattern::Neutral,
                );
                state.temperature = weather.temperature;
                state.humidity = weather.humidity;
                state.wind_speed = weather.wind_speed;
                state.drought_factor = weather.drought_factor;
                update_weather(&mut state);
                println!("Applied Perth Summer preset");
            }
            
            ui.separator();
            
            // Fuel Spawning
            ui.heading("🌳 Add Fuel");
            ui.label("Click on map to place fuel:");
            
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.spawn_fuel_type, FuelType::DryGrass, "🌾 Grass");
                ui.selectable_value(&mut state.spawn_fuel_type, FuelType::StringyBark, "🌲 Stringy");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.spawn_fuel_type, FuelType::SmoothBark, "🌳 Smooth");
                ui.selectable_value(&mut state.spawn_fuel_type, FuelType::Shrubland, "🌿 Shrub");
            });
            ui.selectable_value(&mut state.spawn_fuel_type, FuelType::DeadWood, "🪵 Dead Wood");
            
            ui.separator();
            
            // Quick Actions
            ui.heading("⚡ Actions");
            
            if ui.button("🌾 Add Grass Field (5x5)").clicked() {
                add_grass_field(&mut state.simulation, 0.0, 0.0, 5.0, 5.0);
                println!("Added 5x5m grass field at center");
            }
            
            if ui.button("🌲 Add Stringybark Tree").clicked() {
                add_tree(&mut state.simulation, 0.0, 0.0, Fuel::eucalyptus_stringybark());
                println!("Added stringybark tree at center");
            }
            
            if ui.button("🔥 Ignite Center").clicked() {
                ignite_at_position(&mut state.simulation, 0.0, 0.0, 5.0);
                println!("Ignited elements at center");
            }
            
            ui.separator();
            
            // Simulation Controls
            if ui.button(if state.paused { "▶ Resume" } else { "⏸ Pause" }).clicked() {
                state.paused = !state.paused;
            }
            
            if ui.button("🔄 Reset").clicked() {
                let mut new_sim = FireSimulation::new(500.0, 500.0, 100.0);
                let weather = WeatherSystem::new(
                    state.temperature,
                    state.humidity,
                    state.wind_speed,
                    state.wind_direction,
                    state.drought_factor,
                );
                new_sim.set_weather(weather);
                state.simulation = new_sim;
                state.time = 0.0;
                println!("Simulation reset");
            }
            
            ui.separator();
            ui.label("Controls:");
            ui.label("Left Click: Add fuel");
            ui.label("Right Click: Ignite");
            ui.label("Arrow Keys: Pan");
            ui.label("+/-: Zoom");
            ui.label("Space: Pause");
            ui.label("R: Reset");
        });
    
    // Handle mouse clicks for adding fuel and igniting
    if let (Ok((camera, camera_transform)), Ok(window)) = (camera_query.get_single(), windows.get_single()) {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                let sim_x = world_pos.x / SCALE_FACTOR;
                let sim_y = world_pos.y / SCALE_FACTOR;
                
                // Left click to add fuel
                if mouse_button.just_pressed(MouseButton::Left) {
                    add_fuel_at_cursor(&mut state, sim_x, sim_y);
                }
                
                // Right click to ignite
                if mouse_button.just_pressed(MouseButton::Right) {
                    ignite_at_position(&mut state.simulation, sim_x, sim_y, 10.0);
                    println!("Ignited elements at ({:.1}, {:.1})", sim_x, sim_y);
                }
            }
        }
    }
}

fn update_weather(state: &mut SimulationState) {
    let weather = WeatherSystem::new(
        state.temperature,
        state.humidity,
        state.wind_speed,
        state.wind_direction,
        state.drought_factor,
    );
    state.ffdi = weather.calculate_ffdi();
    state.fire_danger_rating = weather.fire_danger_rating().to_string();
    state.simulation.set_weather(weather);
}

fn add_fuel_at_cursor(state: &mut SimulationState, x: f32, y: f32) {
    let fuel = match state.spawn_fuel_type {
        FuelType::DryGrass => Fuel::dry_grass(),
        FuelType::StringyBark => Fuel::eucalyptus_stringybark(),
        FuelType::SmoothBark => Fuel::eucalyptus_smooth_bark(),
        FuelType::Shrubland => Fuel::shrubland(),
        FuelType::DeadWood => Fuel::dead_wood_litter(),
    };
    
    let part = match state.spawn_fuel_type {
        FuelType::DryGrass => FuelPart::GroundVegetation,
        FuelType::DeadWood => FuelPart::GroundLitter,
        FuelType::Shrubland => FuelPart::GroundVegetation,
        _ => FuelPart::TrunkLower,
    };
    
    state.simulation.add_fuel_element(
        SimVec3::new(x, y, 0.0),
        fuel,
        1.0,
        part,
        None,
    );
}

fn add_grass_field(sim: &mut FireSimulation, center_x: f32, center_y: f32, width: f32, height: f32) {
    let grass = Fuel::dry_grass();
    let spacing = 1.0;
    
    let x_start = center_x - width / 2.0;
    let x_end = center_x + width / 2.0;
    let y_start = center_y - height / 2.0;
    let y_end = center_y + height / 2.0;
    
    let mut x = x_start;
    while x <= x_end {
        let mut y = y_start;
        while y <= y_end {
            sim.add_fuel_element(
                SimVec3::new(x, y, 0.0),
                grass.clone(),
                0.5,
                FuelPart::GroundVegetation,
                None,
            );
            y += spacing;
        }
        x += spacing;
    }
}

fn add_tree(sim: &mut FireSimulation, x: f32, y: f32, fuel: Fuel) {
    let base_id = sim.add_fuel_element(
        SimVec3::new(x, y, 0.0),
        fuel.clone(),
        10.0,
        FuelPart::TrunkLower,
        None,
    );
    
    sim.add_fuel_element(
        SimVec3::new(x, y, 5.0),
        fuel.clone(),
        8.0,
        FuelPart::TrunkMiddle,
        Some(base_id),
    );
    
    sim.add_fuel_element(
        SimVec3::new(x, y, 10.0),
        fuel.clone(),
        6.0,
        FuelPart::TrunkUpper,
        Some(base_id),
    );
    
    sim.add_fuel_element(
        SimVec3::new(x, y, 15.0),
        fuel,
        5.0,
        FuelPart::Crown,
        Some(base_id),
    );
}

fn ignite_at_position(sim: &mut FireSimulation, x: f32, y: f32, radius: f32) {
    for id in 0..sim.element_count() as u32 {
        if let Some(element) = sim.get_element(id) {
            let dx = element.position.x - x;
            let dy = element.position.y - y;
            let dist = (dx * dx + dy * dy).sqrt();
            
            if dist <= radius {
                sim.ignite_element(id, 600.0);
            }
        }
    }
}

fn update_fire_visualization(
    mut commands: Commands,
    state: Res<SimulationState>,
    fire_query: Query<(Entity, &FireElement)>,
) {
    // Remove old sprites
    for (entity, _) in fire_query.iter() {
        commands.entity(entity).despawn();
    }
    
    // Create new sprites for burning elements
    for id in 0..state.simulation.element_count() as u32 {
        if let Some(element) = state.simulation.get_element(id) {
            if element.ignited && element.temperature > 100.0 {
                let x = element.position.x * SCALE_FACTOR;
                let y = element.position.y * SCALE_FACTOR;
                let z = element.position.z * SCALE_FACTOR * 0.1; // Less vertical scale
                
                // Color based on temperature
                let temp_normalized = ((element.temperature - 100.0) / 1000.0).clamp(0.0, 1.0);
                let color = if temp_normalized < 0.3 {
                    FIRE_COLOR_LOW
                } else if temp_normalized < 0.7 {
                    FIRE_COLOR_MEDIUM
                } else {
                    FIRE_COLOR_HIGH
                };
                
                // Size based on flame height and fuel remaining
                let size = (element.flame_height * 2.0 + 1.0).min(10.0) * SCALE_FACTOR;
                
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color,
                            custom_size: Some(Vec2::new(size, size)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, y, z),
                        ..default()
                    },
                    FireElement { element_id: id },
                ));
            } else if element.temperature > 50.0 {
                // Show heated but not burning elements as dim
                let x = element.position.x * SCALE_FACTOR;
                let y = element.position.y * SCALE_FACTOR;
                
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(0.8, 0.8, 0.0, 0.3),
                            custom_size: Some(Vec2::new(1.0, 1.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, y, 0.0),
                        ..default()
                    },
                    FireElement { element_id: id },
                ));
            }
        }
    }
}

fn update_ember_visualization(
    mut commands: Commands,
    state: Res<SimulationState>,
    ember_query: Query<(Entity, &EmberSprite)>,
) {
    // Remove old sprites
    for (entity, _) in ember_query.iter() {
        commands.entity(entity).despawn();
    }
    
    // Create sprites for embers (sample some if too many)
    let ember_sample_rate = if state.ember_count > 500 { 3 } else { 1 };
    
    for (idx, ember) in state.simulation.get_embers().iter().enumerate() {
        if idx % ember_sample_rate != 0 {
            continue;
        }
        
        if ember.temperature > 200.0 {
            let x = ember.position.x * SCALE_FACTOR;
            let y = ember.position.y * SCALE_FACTOR;
            let z = ember.position.z * SCALE_FACTOR * 0.1;
            
            let temp_factor = ((ember.temperature - 200.0) / 800.0).clamp(0.0, 1.0);
            let color = Color::rgba(1.0, 0.5 + temp_factor * 0.5, 0.0, 0.8);
            let size = 0.5 + temp_factor * 1.5;
            
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::new(size, size)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, z),
                    ..default()
                },
                EmberSprite { ember_index: idx },
            ));
        }
    }
}
