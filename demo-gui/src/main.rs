//! Bevy-based GUI demo for the Australia Fire Simulation
//!
//! This demo provides a real-time 3D visualization of the fire simulation with interactive controls.

use bevy::prelude::*;
use fire_sim_core::{
    FireSimulation, Fuel, FuelPart, SuppressionAgent, SuppressionDroplet, TerrainData, Vec3 as SimVec3,
    WeatherSystem,
};

/// Configuration for the simulation demo
#[derive(Clone, Resource)]
pub struct DemoConfig {
    // Map settings
    pub map_width: f32,
    pub map_height: f32,
    pub terrain_type: TerrainType,
    
    // Fire settings
    pub elements_x: usize,
    pub elements_y: usize,
    pub fuel_mass: f32,
    pub fuel_type: FuelType,
    pub initial_ignitions: usize,
    pub spacing: f32,
    
    // Weather settings
    pub temperature: f32,
    pub humidity: f32,
    pub wind_speed: f32,
    pub wind_direction: f32,
    pub drought_factor: f32,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            map_width: 200.0,
            map_height: 200.0,
            terrain_type: TerrainType::Hill,
            elements_x: 10,
            elements_y: 10,
            fuel_mass: 5.0,
            fuel_type: FuelType::DryGrass,
            initial_ignitions: 5,
            spacing: 8.0,
            temperature: 35.0,
            humidity: 0.15,
            wind_speed: 20.0,
            wind_direction: 45.0,
            drought_factor: 10.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TerrainType {
    Flat,
    Hill,
    Valley,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FuelType {
    DryGrass,
    EucalyptusStringybark,
    EucalyptusSmoothBark,
    Shrubland,
    DeadWood,
}

impl FuelType {
    fn to_fuel(&self) -> Fuel {
        match self {
            FuelType::DryGrass => Fuel::dry_grass(),
            FuelType::EucalyptusStringybark => Fuel::eucalyptus_stringybark(),
            FuelType::EucalyptusSmoothBark => Fuel::eucalyptus_smooth_bark(),
            FuelType::Shrubland => Fuel::shrubland(),
            FuelType::DeadWood => Fuel::dead_wood_litter(),
        }
    }
}

/// Main entry point
fn main() {
    // You can customize the configuration here
    let config = DemoConfig::default();
    
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Australia Fire Simulation - Bevy Demo".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(config)
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

impl FromWorld for SimulationState {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<DemoConfig>().clone();
        
        // Create terrain based on config
        let terrain = match config.terrain_type {
            TerrainType::Flat => TerrainData::flat(config.map_width, config.map_height, 5.0, 0.0),
            TerrainType::Hill => TerrainData::single_hill(config.map_width, config.map_height, 5.0, 0.0, 80.0, 40.0),
            TerrainType::Valley => TerrainData::valley_between_hills(config.map_width, config.map_height, 5.0, 0.0, 80.0),
        };
        
        // Create simulation
        let mut sim = FireSimulation::new(5.0, terrain);
        
        // Set weather conditions from config
        let weather = WeatherSystem::new(
            config.temperature,
            config.humidity,
            config.wind_speed,
            config.wind_direction,
            config.drought_factor,
        );
        sim.set_weather(weather);
        
        // Add fuel elements in a grid
        let center_x = config.map_width / 2.0;
        let center_y = config.map_height / 2.0;
        let start_x = center_x - (config.elements_x as f32 * config.spacing) / 2.0;
        let start_y = center_y - (config.elements_y as f32 * config.spacing) / 2.0;
        
        for i in 0..config.elements_x {
            for j in 0..config.elements_y {
                let x = start_x + i as f32 * config.spacing;
                let y = start_y + j as f32 * config.spacing;
                let elevation = sim.get_terrain().elevation_at(x, y);
                
                let fuel = config.fuel_type.to_fuel();
                sim.add_fuel_element(
                    SimVec3::new(x, y, elevation + 0.5),
                    fuel,
                    config.fuel_mass,
                    FuelPart::GroundVegetation,
                    None,
                );
            }
        }
        
        // Ignite elements based on config
        for id in 0..config.initial_ignitions.min(config.elements_x * config.elements_y) {
            sim.ignite_element(id as u32, 600.0);
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
    terrain: &TerrainData,
) {
    // Create a 3D terrain mesh with actual elevation data
    let resolution = 5.0; // Sample every 5 meters
    let nx = (terrain.width / resolution) as usize + 1;
    let ny = (terrain.height / resolution) as usize + 1;
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices with elevation
    for iy in 0..ny {
        for ix in 0..nx {
            let x = ix as f32 * resolution;
            let y = iy as f32 * resolution;
            let z = terrain.elevation_at(x, y);
            
            positions.push([x, z, y]); // Bevy uses Y-up, so we map: x→x, elevation→y, y→z
            normals.push([0.0, 1.0, 0.0]); // Will recalculate proper normals below
            uvs.push([x / terrain.width, y / terrain.height]);
        }
    }
    
    // Generate triangles
    for iy in 0..(ny - 1) {
        for ix in 0..(nx - 1) {
            let i0 = (iy * nx + ix) as u32;
            let i1 = (iy * nx + ix + 1) as u32;
            let i2 = ((iy + 1) * nx + ix) as u32;
            let i3 = ((iy + 1) * nx + ix + 1) as u32;
            
            // Two triangles per quad
            indices.push(i0);
            indices.push(i2);
            indices.push(i1);
            
            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
        }
    }
    
    // Calculate proper normals
    for i in 0..positions.len() {
        normals[i] = [0.0, 0.0, 0.0];
    }
    
    for face in indices.chunks(3) {
        let i0 = face[0] as usize;
        let i1 = face[1] as usize;
        let i2 = face[2] as usize;
        
        let v0 = Vec3::from_array(positions[i0]);
        let v1 = Vec3::from_array(positions[i1]);
        let v2 = Vec3::from_array(positions[i2]);
        
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize();
        
        // Accumulate normals for each vertex
        normals[i0][0] += normal.x;
        normals[i0][1] += normal.y;
        normals[i0][2] += normal.z;
        normals[i1][0] += normal.x;
        normals[i1][1] += normal.y;
        normals[i1][2] += normal.z;
        normals[i2][0] += normal.x;
        normals[i2][1] += normal.y;
        normals[i2][2] += normal.z;
    }
    
    // Normalize the accumulated normals
    for normal in &mut normals {
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 0.0 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        }
    }
    
    // Create mesh
    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    
    let terrain_mesh = meshes.add(mesh);
    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });
    
    commands.spawn(PbrBundle {
        mesh: terrain_mesh,
        material: terrain_material,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
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
    config: Res<DemoConfig>,
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
    
    // Reset simulation - recreate from config
    if keyboard.just_pressed(KeyCode::KeyR) {
        // Create terrain based on config
        let terrain = match config.terrain_type {
            TerrainType::Flat => TerrainData::flat(config.map_width, config.map_height, 5.0, 0.0),
            TerrainType::Hill => TerrainData::single_hill(config.map_width, config.map_height, 5.0, 0.0, 80.0, 40.0),
            TerrainType::Valley => TerrainData::valley_between_hills(config.map_width, config.map_height, 5.0, 0.0, 80.0),
        };
        
        // Create simulation
        let mut sim = FireSimulation::new(5.0, terrain);
        
        // Set weather conditions from config
        let weather = WeatherSystem::new(
            config.temperature,
            config.humidity,
            config.wind_speed,
            config.wind_direction,
            config.drought_factor,
        );
        sim.set_weather(weather);
        
        // Add fuel elements in a grid
        let center_x = config.map_width / 2.0;
        let center_y = config.map_height / 2.0;
        let start_x = center_x - (config.elements_x as f32 * config.spacing) / 2.0;
        let start_y = center_y - (config.elements_y as f32 * config.spacing) / 2.0;
        
        for i in 0..config.elements_x {
            for j in 0..config.elements_y {
                let x = start_x + i as f32 * config.spacing;
                let y = start_y + j as f32 * config.spacing;
                let elevation = sim.get_terrain().elevation_at(x, y);
                
                let fuel = config.fuel_type.to_fuel();
                sim.add_fuel_element(
                    SimVec3::new(x, y, elevation + 0.5),
                    fuel,
                    config.fuel_mass,
                    FuelPart::GroundVegetation,
                    None,
                );
            }
        }
        
        // Ignite elements based on config
        for id in 0..config.initial_ignitions.min(config.elements_x * config.elements_y) {
            sim.ignite_element(id as u32, 600.0);
        }
        
        sim_state.simulation = sim;
        sim_state.paused = false;
        sim_state.speed = 1.0;
        sim_state.time_accumulator = 0.0;
        // Note: Fuel entity map is not cleared - entities will be updated in update_fuel_visuals
    }
    
    // Add water suppression
    if keyboard.just_pressed(KeyCode::KeyW) {
        let center_x = config.map_width / 2.0;
        let center_y = config.map_height / 2.0;
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
