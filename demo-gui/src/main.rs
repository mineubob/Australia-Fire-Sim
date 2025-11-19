//! Bevy-based GUI demo for the Australia Fire Simulation
//!
//! This demo provides a real-time 3D visualization of the fire simulation with interactive controls.

use bevy::prelude::*;
use fire_sim_core::{
    FireSimulation, Fuel, FuelPart, SuppressionAgent, SuppressionDroplet, TerrainData, Vec3 as SimVec3,
    WeatherSystem,
};

/// Main entry point
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Australia Fire Simulation - Bevy Demo".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<SimulationState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            update_simulation,
            update_fuel_visuals,
            update_camera_controls,
            update_ui,
            handle_controls,
        ))
        .run();
}

/// Simulation state resource
#[derive(Resource)]
struct SimulationState {
    simulation: FireSimulation,
    paused: bool,
    speed: f32,
    time_accumulator: f32,
    fuel_entity_map: Vec<(u32, Entity)>, // Maps fuel element ID to Bevy entity
}

impl Default for SimulationState {
    fn default() -> Self {
        // Create terrain with a hill
        let map_width = 200.0;
        let map_height = 200.0;
        let terrain = TerrainData::single_hill(map_width, map_height, 5.0, 0.0, 80.0, 40.0);
        
        // Create simulation
        let mut sim = FireSimulation::new(5.0, terrain);
        
        // Set weather conditions - extreme fire danger
        let weather = WeatherSystem::new(
            35.0, // 35°C temperature
            0.15, // 15% humidity
            20.0, // 20 m/s wind speed
            45.0, // 45° wind direction (NE)
            10.0, // Very high drought factor
        );
        sim.set_weather(weather);
        
        // Add fuel elements in a grid
        let elements_x = 10;
        let elements_y = 10;
        let fuel_mass = 5.0;
        let center_x = map_width / 2.0;
        let center_y = map_height / 2.0;
        let spacing = 8.0;
        let start_x = center_x - (elements_x as f32 * spacing) / 2.0;
        let start_y = center_y - (elements_y as f32 * spacing) / 2.0;
        
        for i in 0..elements_x {
            for j in 0..elements_y {
                let x = start_x + i as f32 * spacing;
                let y = start_y + j as f32 * spacing;
                let elevation = sim.get_terrain().elevation_at(x, y); // Will be set by terrain
                
                let fuel = Fuel::dry_grass();
                sim.add_fuel_element(
                    SimVec3::new(x, y, elevation + 0.5),
                    fuel,
                    fuel_mass,
                    FuelPart::GroundVegetation,
                    None,
                );
            }
        }
        
        // Ignite a few elements at the center
        for id in 0..5 {
            sim.ignite_element(id, 600.0);
        }
        
        Self {
            simulation: sim,
            paused: false,
            speed: 1.0,
            time_accumulator: 0.0,
            fuel_entity_map: Vec::new(),
        }
    }
}

/// Marker component for fuel element visuals
#[derive(Component)]
struct FuelVisual {
    element_id: u32,
}

/// Marker component for the camera
#[derive(Component)]
struct MainCamera;

/// UI text marker components
#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct ControlsText;

/// Setup the 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sim_state: ResMut<SimulationState>,
) {
    // Add light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    
    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
    
    // Add camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(150.0, 120.0, 150.0)
                .looking_at(Vec3::new(100.0, 0.0, 100.0), Vec3::Y),
            ..default()
        },
        MainCamera,
    ));
    
    // Create terrain mesh
    let terrain = &sim_state.simulation.grid.terrain;
    spawn_terrain(&mut commands, &mut meshes, &mut materials, terrain);
    
    // Spawn fuel element visuals
    let cube_mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    
    // Collect elements first to avoid borrow issues
    let elements: Vec<_> = sim_state.simulation.get_all_elements()
        .into_iter()
        .map(|e| (e.id, e.position, e.is_ignited()))
        .collect();
    
    for (element_id, pos, is_ignited) in elements {
        let material = if is_ignited {
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.3, 0.0),
                emissive: LinearRgba::rgb(5.0, 1.0, 0.0),
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.6, 0.2),
                ..default()
            })
        };
        
        let entity = commands.spawn((
            PbrBundle {
                mesh: cube_mesh.clone(),
                material,
                transform: Transform::from_xyz(pos.x, pos.z, pos.y),
                ..default()
            },
            FuelVisual {
                element_id,
            },
        )).id();
        
        sim_state.fuel_entity_map.push((element_id, entity));
    }
    
    // Setup UI
    setup_ui(&mut commands);
}

fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    _terrain: &TerrainData,
) {
    // Create a simple terrain plane for visualization
    let terrain_mesh = meshes.add(Plane3d::default().mesh().size(200.0, 200.0));
    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });
    
    commands.spawn(PbrBundle {
        mesh: terrain_mesh,
        material: terrain_material,
        transform: Transform::from_xyz(100.0, 0.0, 100.0),
        ..default()
    });
}

fn setup_ui(commands: &mut Commands) {
    // Root UI container
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Australia Fire Simulation - Bevy Demo",
                TextStyle {
                    font_size: 32.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            
            // Stats text
            parent.spawn((
                TextBundle::from_section(
                    "Initializing...",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::srgb(0.8, 0.8, 0.8),
                        ..default()
                    },
                ),
                StatsText,
            ));
            
            // Controls text
            parent.spawn((
                TextBundle::from_section(
                    "Controls:\n  SPACE - Pause/Resume\n  [ / ] - Speed Down/Up\n  R - Reset\n  W - Add Water Suppression\n  Arrow Keys - Camera",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.6, 0.6, 0.6),
                        ..default()
                    },
                ),
                ControlsText,
            ));
        });
}

/// Update the simulation
fn update_simulation(
    time: Res<Time>,
    mut sim_state: ResMut<SimulationState>,
) {
    if sim_state.paused {
        return;
    }
    
    // Accumulate time with speed multiplier
    sim_state.time_accumulator += time.delta_seconds() * sim_state.speed;
    
    // Update simulation at 10 FPS (0.1 second timesteps)
    let timestep = 0.1;
    while sim_state.time_accumulator >= timestep {
        sim_state.simulation.update(timestep);
        sim_state.time_accumulator -= timestep;
    }
}

/// Update fuel element visuals based on simulation state
fn update_fuel_visuals(
    sim_state: Res<SimulationState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&FuelVisual, &Handle<StandardMaterial>)>,
) {
    for (fuel_visual, material_handle) in query.iter() {
        if let Some(element) = sim_state.simulation.get_all_elements().into_iter().find(|e| e.id == fuel_visual.element_id) {
            if let Some(material) = materials.get_mut(material_handle) {
                if element.is_ignited() {
                    // Calculate color based on temperature
                    let temp_factor = (element.temperature() / 1200.0).clamp(0.0, 1.0);
                    material.base_color = Color::srgb(1.0, 0.5 - temp_factor * 0.3, 0.0);
                    material.emissive = LinearRgba::rgb(
                        5.0 * temp_factor,
                        2.0 * temp_factor,
                        0.0
                    );
                } else if element.fuel_remaining() < 0.1 {
                    // Burnt out
                    material.base_color = Color::srgb(0.1, 0.1, 0.1);
                    material.emissive = LinearRgba::rgb(0.0, 0.0, 0.0);
                } else {
                    // Unburnt fuel
                    material.base_color = Color::srgb(0.2, 0.6, 0.2);
                    material.emissive = LinearRgba::rgb(0.0, 0.0, 0.0);
                }
            }
        }
    }
}

/// Camera controls
fn update_camera_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let mut transform = query.single_mut();
    let speed = 50.0 * time.delta_seconds();
    let rotation_speed = 1.0 * time.delta_seconds();
    
    // Movement
    if keyboard.pressed(KeyCode::ArrowUp) {
        let forward = transform.forward();
        transform.translation += forward.as_vec3() * speed;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        let forward = transform.forward();
        transform.translation -= forward.as_vec3() * speed;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        transform.rotate_y(rotation_speed);
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        transform.rotate_y(-rotation_speed);
    }
}

/// Update UI text
fn update_ui(
    sim_state: Res<SimulationState>,
    mut query: Query<&mut Text, With<StatsText>>,
) {
    let stats = sim_state.simulation.get_stats();
    let weather = &sim_state.simulation.weather;
    
    for mut text in query.iter_mut() {
        text.sections[0].value = format!(
            "Time: {:.1}s | Burning: {} | Fuel Consumed: {:.1} kg | Max Temp: {:.0}°C\n\
             Weather: {:.0}°C, {:.0}% RH, {:.1} m/s wind | FFDI: {:.1} ({})\n\
             Status: {} | Speed: {:.1}x",
            stats.simulation_time,
            stats.burning_elements,
            stats.total_fuel_consumed,
            sim_state.simulation.get_all_elements().iter()
                .map(|e| e.temperature())
                .fold(0.0f32, f32::max),
            weather.temperature,
            weather.humidity * 100.0,
            weather.wind_speed,
            weather.calculate_ffdi(),
            weather.fire_danger_rating(),
            if sim_state.paused { "PAUSED" } else { "RUNNING" },
            sim_state.speed,
        );
    }
}

/// Handle user controls
fn handle_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimulationState>,
) {
    // Pause/Resume
    if keyboard.just_pressed(KeyCode::Space) {
        sim_state.paused = !sim_state.paused;
    }
    
    // Speed controls
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        sim_state.speed = (sim_state.speed * 0.5).max(0.1);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        sim_state.speed = (sim_state.speed * 2.0).min(10.0);
    }
    
    // Reset simulation
    if keyboard.just_pressed(KeyCode::KeyR) {
        *sim_state = SimulationState::default();
    }
    
    // Add water suppression
    if keyboard.just_pressed(KeyCode::KeyW) {
        let center_x = 100.0;
        let center_y = 100.0;
        let drop_altitude = 60.0;
        
        // Add water droplets in a circular pattern
        for i in 0..30 {
            let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
            let radius = 25.0;
            let droplet = SuppressionDroplet::new(
                SimVec3::new(
                    center_x + angle.cos() * radius,
                    center_y + angle.sin() * radius,
                    drop_altitude,
                ),
                SimVec3::new(0.0, 0.0, -5.0),
                10.0,
                SuppressionAgent::Water,
            );
            sim_state.simulation.add_suppression_droplet(droplet);
        }
    }
}
