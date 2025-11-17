use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use fire_sim_core::{
    ClimatePattern, FireSimulation, Fuel, FuelPart, Vec3 as SimVec3, WeatherPreset, WeatherSystem,
};

// Constants
const SCALE_FACTOR: f32 = 1.0; // Scale simulation coordinates to Bevy world
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
        .add_systems(
            Update,
            (
                ui_system,
                update_simulation,
                update_fire_visualization,
                update_ember_visualization,
                update_cloud_visualization,
                camera_controls,
                fps_counter_system,
            ),
        )
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
    field_spacing: f32,
    bulk_add_mode: bool,
    bulk_first_point: Option<(f32, f32)>,

    // Statistics
    total_elements: usize,
    burning_elements: usize,
    ember_count: usize,
    fuel_consumed: f32,
    ffdi: f32,
    fire_danger_rating: String,

    // Performance metrics
    fps: f32,
    frame_time_ms: f32,
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
            field_spacing: 1.0,
            bulk_add_mode: false,
            bulk_first_point: None,
            total_elements: 0,
            burning_elements: 0,
            ember_count: 0,
            fuel_consumed: 0.0,
            ffdi: weather.calculate_ffdi(),
            fire_danger_rating: weather.fire_danger_rating().to_string(),
            fps: 60.0,
            frame_time_ms: 16.7,
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

// Component to mark cloud sprites
#[derive(Component)]
#[allow(dead_code)]
struct CloudSprite {
    cloud_index: usize,
}

fn setup(mut commands: Commands) {
    // Setup 2D camera - default scale for good initial view
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
        camera_transform.scale.x = camera_transform.scale.x.max(1.0);
        camera_transform.scale.y = camera_transform.scale.y.max(1.0);
        camera_transform.scale.z = camera_transform.scale.z.max(1.0);
    }
    if keyboard.pressed(KeyCode::Minus) || keyboard.pressed(KeyCode::NumpadSubtract) {
        camera_transform.scale *= 1.0 + zoom_speed * time.delta_seconds();
        camera_transform.scale.x = camera_transform.scale.x.min(3.0);
        camera_transform.scale.y = camera_transform.scale.y.min(3.0);
        camera_transform.scale.z = camera_transform.scale.z.min(3.0);
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
        println!(
            "Simulation {}",
            if state.paused { "paused" } else { "resumed" }
        );
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("🔥 Fire Simulation Control");
                ui.separator();

                // Status
                ui.label(format!("Time: {:.1}s", state.time));
                ui.label(format!(
                    "Status: {}",
                    if state.paused {
                        "⏸ Paused"
                    } else {
                        "▶ Running"
                    }
                ));
                ui.separator();

                // Statistics
                ui.heading("📊 Statistics");
                ui.label(format!("FPS: {:.1}", state.fps));
                ui.label(format!("Frame Time: {:.2}ms", state.frame_time_ms));
                ui.label(format!("Total Elements: {}", state.total_elements));
                
                // Show sampling info if elements are being sampled for performance
                let sample_rate = if state.total_elements > 100000 {
                    10
                } else if state.total_elements > 50000 {
                    5
                } else if state.total_elements > 20000 {
                    3
                } else if state.total_elements > 10000 {
                    2
                } else {
                    1
                };
                
                if sample_rate > 1 {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 200, 100),
                        format!("⚡ Rendering 1/{} elements for performance", sample_rate)
                    );
                }
                
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

                ui.colored_label(
                    danger_color,
                    format!("Rating: {}", state.fire_danger_rating),
                );
                ui.separator();

                // Wind Compass
                ui.heading("🧭 Wind Direction");
                let compass_size = 80.0;
                let (response, painter) = ui.allocate_painter(
                    egui::Vec2::new(compass_size, compass_size),
                    egui::Sense::hover(),
                );
                let center = response.rect.center();
                let radius = compass_size / 2.0 - 5.0;

                // Draw compass circle
                painter.circle_stroke(center, radius, egui::Stroke::new(2.0, egui::Color32::GRAY));

                // Draw N, S, E, W markers
                let text_radius = radius + 10.0;
                painter.text(
                    egui::pos2(center.x, center.y - text_radius),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
                painter.text(
                    egui::pos2(center.x, center.y + text_radius),
                    egui::Align2::CENTER_CENTER,
                    "S",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
                painter.text(
                    egui::pos2(center.x + text_radius, center.y),
                    egui::Align2::CENTER_CENTER,
                    "E",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
                painter.text(
                    egui::pos2(center.x - text_radius, center.y),
                    egui::Align2::CENTER_CENTER,
                    "W",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );

                // Convert wind direction to radians (0° = North, clockwise)
                let wind_angle_rad = state.wind_direction.to_radians();

                // Calculate arrow length based on wind speed (scale to fit compass)
                let arrow_length = (state.wind_speed / 100.0).min(1.0) * radius * 0.8;

                // Calculate arrow end point
                let arrow_end = egui::pos2(
                    center.x + arrow_length * wind_angle_rad.sin(),
                    center.y - arrow_length * wind_angle_rad.cos(),
                );

                // Draw wind arrow
                painter.arrow(
                    center,
                    arrow_end - center,
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 150, 255)),
                );

                // Display wind speed
                ui.label(format!("Speed: {:.1} km/h", state.wind_speed));
                ui.label(format!("Direction: {:.0}°", state.wind_direction));
                ui.separator();

                // Weather Controls
                ui.heading("🌤️ Weather");

                ui.label("Temperature (°C):");
                if ui
                    .add(egui::Slider::new(&mut state.temperature, 10.0..=50.0))
                    .changed()
                {
                    update_weather(&mut state);
                }

                ui.label("Humidity (%):");
                if ui
                    .add(egui::Slider::new(&mut state.humidity, 10.0..=90.0))
                    .changed()
                {
                    update_weather(&mut state);
                }

                ui.label("Wind Speed (km/h):");
                if ui
                    .add(egui::Slider::new(&mut state.wind_speed, 0.0..=100.0))
                    .changed()
                {
                    update_weather(&mut state);
                }

                ui.label("Wind Direction (°):");
                if ui
                    .add(egui::Slider::new(&mut state.wind_direction, 0.0..=360.0))
                    .changed()
                {
                    update_weather(&mut state);
                }

                ui.label("Drought Factor (0-10):");
                if ui
                    .add(egui::Slider::new(&mut state.drought_factor, 0.0..=10.0))
                    .changed()
                {
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
                        15,   // January 15
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
                    ui.selectable_value(
                        &mut state.spawn_fuel_type,
                        FuelType::StringyBark,
                        "🌲 Stringy",
                    );
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut state.spawn_fuel_type,
                        FuelType::SmoothBark,
                        "🌳 Smooth",
                    );
                    ui.selectable_value(
                        &mut state.spawn_fuel_type,
                        FuelType::Shrubland,
                        "🌿 Shrub",
                    );
                });
                ui.selectable_value(
                    &mut state.spawn_fuel_type,
                    FuelType::DeadWood,
                    "🪵 Dead Wood",
                );

                ui.separator();

                // Bulk Fuel Addition
                ui.heading("📦 Bulk Add Fuel");
                ui.label("Select two corners to define area:");
                ui.horizontal(|ui| {
                    ui.label("Spacing:");
                    ui.add(egui::Slider::new(&mut state.field_spacing, 0.5..=5.0).step_by(0.5));
                });

                let button_text = if state.bulk_add_mode {
                    if state.bulk_first_point.is_some() {
                        "✓ Click second corner"
                    } else {
                        "✓ Click first corner"
                    }
                } else {
                    "📍 Select Area"
                };

                if ui.button(button_text).clicked() {
                    state.bulk_add_mode = !state.bulk_add_mode;
                    state.bulk_first_point = None; // Reset selection
                    if state.bulk_add_mode {
                        println!("Bulk add mode enabled - click first corner");
                    } else {
                        println!("Bulk add mode disabled");
                    }
                }

                ui.separator();

                // Quick Actions
                ui.heading("⚡ Actions");

                if ui.button("🌲 Add Stringybark Tree").clicked() {
                    add_tree(
                        &mut state.simulation,
                        0.0,
                        0.0,
                        Fuel::eucalyptus_stringybark(),
                    );
                    println!("Added stringybark tree at center");
                }

                if ui.button("🔥 Ignite Center").clicked() {
                    ignite_at_position(&mut state.simulation, 0.0, 0.0, 5.0);
                    println!("Ignited elements at center");
                }

                ui.separator();

                // Simulation Controls
                if ui
                    .button(if state.paused {
                        "▶ Resume"
                    } else {
                        "⏸ Pause"
                    })
                    .clicked()
                {
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
                if state.bulk_add_mode {
                    if state.bulk_first_point.is_some() {
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 200, 100),
                            "Left Click: Second corner",
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 200, 100),
                            "Left Click: First corner",
                        );
                    }
                } else {
                    ui.label("Left Click: Add fuel");
                }
                ui.label("Right Click: Ignite");
                ui.label("Arrow Keys: Pan");
                ui.label("+/-: Zoom");
                ui.label("Space: Pause");
                ui.label("R: Reset");
            }); // End of ScrollArea
        }); // End of SidePanel

    // Handle mouse clicks for adding fuel and igniting
    if let (Ok((camera, camera_transform)), Ok(window)) =
        (camera_query.get_single(), windows.get_single())
    {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                let sim_x = world_pos.x / SCALE_FACTOR;
                let sim_y = world_pos.y / SCALE_FACTOR;

                // Left click to add fuel
                if mouse_button.just_pressed(MouseButton::Left) {
                    if state.bulk_add_mode {
                        if let Some((first_x, first_y)) = state.bulk_first_point {
                            // Second click - calculate area and place field
                            let width = (sim_x - first_x).abs();
                            let height = (sim_y - first_y).abs();
                            let center_x = (first_x + sim_x) / 2.0;
                            let center_y = (first_y + sim_y) / 2.0;
                            
                            let spacing = state.field_spacing;
                            let fuel_type = state.spawn_fuel_type;
                            add_fuel_field(
                                &mut state.simulation,
                                center_x,
                                center_y,
                                width,
                                height,
                                spacing,
                                fuel_type,
                            );
                            let fuel_name = match fuel_type {
                                FuelType::DryGrass => "grass",
                                FuelType::StringyBark => "stringybark",
                                FuelType::SmoothBark => "smooth bark",
                                FuelType::Shrubland => "shrubland",
                                FuelType::DeadWood => "dead wood",
                            };
                            println!(
                                "Added {:.1}x{:.1}m {} field (spacing: {}m) from ({:.1}, {:.1}) to ({:.1}, {:.1})",
                                width, height, fuel_name, spacing, first_x, first_y, sim_x, sim_y
                            );
                            state.bulk_first_point = None; // Reset for next selection
                            state.bulk_add_mode = false; // Exit bulk mode
                        } else {
                            // First click - store position
                            state.bulk_first_point = Some((sim_x, sim_y));
                            println!("First corner selected at ({:.1}, {:.1}), click second corner", sim_x, sim_y);
                        }
                    } else {
                        // Add single fuel element
                        add_fuel_at_cursor(&mut state, sim_x, sim_y);
                    }
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

    state
        .simulation
        .add_fuel_element(SimVec3::new(x, y, 0.0), fuel, 1.0, part, None);
}

fn add_fuel_field(
    sim: &mut FireSimulation,
    center_x: f32,
    center_y: f32,
    width: f32,
    height: f32,
    spacing: f32,
    fuel_type: FuelType,
) {
    let fuel = match fuel_type {
        FuelType::DryGrass => Fuel::dry_grass(),
        FuelType::StringyBark => Fuel::eucalyptus_stringybark(),
        FuelType::SmoothBark => Fuel::eucalyptus_smooth_bark(),
        FuelType::Shrubland => Fuel::shrubland(),
        FuelType::DeadWood => Fuel::dead_wood_litter(),
    };

    let part = match fuel_type {
        FuelType::DryGrass => FuelPart::GroundVegetation,
        FuelType::DeadWood => FuelPart::GroundLitter,
        FuelType::Shrubland => FuelPart::GroundVegetation,
        _ => FuelPart::TrunkLower,
    };

    let x_start = center_x - width / 2.0;
    let x_end = center_x + width / 2.0;
    let y_start = center_y - height / 2.0;
    let y_end = center_y + height / 2.0;

    let mut x = x_start;
    while x <= x_end {
        let mut y = y_start;
        while y <= y_end {
            sim.add_fuel_element(SimVec3::new(x, y, 0.0), fuel.clone(), 0.5, part, None);
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

    // Performance optimization: sample elements when count is very high
    let total_elements = state.simulation.element_count();
    let sample_rate = if total_elements > 100000 {
        10 // Show 1 in 10 elements
    } else if total_elements > 50000 {
        5 // Show 1 in 5 elements
    } else if total_elements > 20000 {
        3 // Show 1 in 3 elements
    } else if total_elements > 10000 {
        2 // Show 1 in 2 elements
    } else {
        1 // Show all elements
    };

    // Create new sprites for elements (with sampling for performance)
    for id in (0..total_elements as u32).step_by(sample_rate) {
        if let Some(element) = state.simulation.get_element(id) {
            let x = element.position.x * SCALE_FACTOR;
            let y = element.position.y * SCALE_FACTOR;
            let z = element.position.z * SCALE_FACTOR * 0.1; // Less vertical scale

            if element.fuel_remaining < 0.01 {
                // Show burnt-out elements as gray ash
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(0.3, 0.3, 0.3, 0.5), // Gray ash
                            custom_size: Some(Vec2::new(2.0, 2.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, y, 0.0),
                        ..default()
                    },
                    FireElement { element_id: id },
                ));
            } else if element.ignited && element.temperature > 100.0 {
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
                // Show heated but not burning elements as dim yellow
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(0.8, 0.8, 0.0, 0.4),
                            custom_size: Some(Vec2::new(2.5, 2.5)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, y, 0.0),
                        ..default()
                    },
                    FireElement { element_id: id },
                ));
            } else {
                // Show unburned fuel elements as small green/brown dots
                let fuel_color = match element.part_type {
                    FuelPart::GroundVegetation => Color::rgba(0.2, 0.6, 0.2, 0.6), // Green for grass
                    FuelPart::GroundLitter => Color::rgba(0.5, 0.3, 0.1, 0.5), // Brown for litter
                    FuelPart::TrunkLower | FuelPart::TrunkMiddle | FuelPart::TrunkUpper => {
                        Color::rgba(0.4, 0.25, 0.1, 0.7)
                    } // Dark brown for trunk
                    FuelPart::Crown => Color::rgba(0.1, 0.5, 0.1, 0.7), // Dark green for crown
                    _ => Color::rgba(0.3, 0.4, 0.2, 0.5),               // Greenish for others
                };

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: fuel_color,
                            custom_size: Some(Vec2::new(3.0, 3.0)),
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

fn fps_counter_system(mut state: ResMut<SimulationState>, time: Res<Time>) {
    // Update FPS and frame time
    state.fps = 1.0 / time.delta_seconds();
    state.frame_time_ms = time.delta_seconds() * 1000.0;
}

fn update_cloud_visualization(
    mut commands: Commands,
    state: Res<SimulationState>,
    cloud_query: Query<(Entity, &CloudSprite)>,
) {
    // Remove old cloud sprites
    for (entity, _) in cloud_query.iter() {
        commands.entity(entity).despawn();
    }

    // Get clouds from simulation
    let clouds = &state.simulation.pyrocb_system.clouds;

    // Create sprites for active clouds
    for (idx, cloud) in clouds.iter().enumerate() {
        if !cloud.active {
            continue;
        }

        let x = cloud.position.x * SCALE_FACTOR;
        let y = cloud.position.y * SCALE_FACTOR;
        // Position clouds at a higher z-layer so they appear above fire
        let z = 100.0;

        // Cloud size based on diameter - allow larger clouds to show expansion
        // Scale down for visibility but don't cap too aggressively
        let size = (cloud.diameter * SCALE_FACTOR * 0.2).max(10.0).min(200.0);

        // Cloud color - white/gray based on charge and age
        let gray_value = 0.8 - (cloud.charge_separation * 0.3);
        let alpha = 0.5 + (cloud.energy / 100000.0).min(0.4);
        let cloud_color = Color::rgba(gray_value, gray_value, gray_value, alpha);

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: cloud_color,
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, z),
                ..default()
            },
            CloudSprite { cloud_index: idx },
        ));

        // Add a darker center to show updraft
        let center_size = size * 0.3;
        let center_gray = gray_value * 0.6;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(center_gray, center_gray, center_gray, alpha * 1.2),
                    custom_size: Some(Vec2::new(center_size, center_size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, z + 0.1),
                ..default()
            },
            CloudSprite { cloud_index: idx },
        ));
    }
}
