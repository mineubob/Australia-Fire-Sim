//! Bevy-based GUI demo for the Australia Fire Simulation
//!
//! This demo provides a real-time 3D visualization of the fire simulation with interactive controls.

use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use fire_sim_core::{
    FireSimulation, Fuel, FuelPart, SuppressionAgent, TerrainData, Vec3 as SimVec3, WeatherPreset,
    WeatherSystem,
};
// sysinfo not needed - using bevy diagnostic plugins instead

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
    pub spacing: f32,

    // Weather settings
    pub use_weather_preset: bool,
    pub weather_preset: WeatherPresetType,
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
            terrain_type: TerrainType::Valley,
            elements_x: 10,
            elements_y: 10,
            fuel_mass: 5.0,
            fuel_type: FuelType::DryGrass,
            spacing: 8.0,
            use_weather_preset: false,
            weather_preset: WeatherPresetType::PerthMetro,
            temperature: 35.0,
            humidity: 15.0,
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

impl TerrainType {
    fn name(self) -> &'static str {
        match self {
            TerrainType::Flat => "Flat",
            TerrainType::Hill => "Hill",
            TerrainType::Valley => "Valley",
        }
    }

    fn all() -> [Self; 3] {
        [TerrainType::Flat, TerrainType::Hill, TerrainType::Valley]
    }
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
    fn to_fuel(self) -> Fuel {
        match self {
            FuelType::DryGrass => Fuel::dry_grass(),
            FuelType::EucalyptusStringybark => Fuel::eucalyptus_stringybark(),
            FuelType::EucalyptusSmoothBark => Fuel::eucalyptus_smooth_bark(),
            FuelType::Shrubland => Fuel::shrubland(),
            FuelType::DeadWood => Fuel::dead_wood_litter(),
        }
    }

    fn name(self) -> &'static str {
        match self {
            FuelType::DryGrass => "Dry Grass",
            FuelType::EucalyptusStringybark => "Eucalyptus Stringybark",
            FuelType::EucalyptusSmoothBark => "Eucalyptus Smooth Bark",
            FuelType::Shrubland => "Shrubland",
            FuelType::DeadWood => "Dead Wood",
        }
    }

    fn all() -> [Self; 5] {
        [
            FuelType::DryGrass,
            FuelType::EucalyptusStringybark,
            FuelType::EucalyptusSmoothBark,
            FuelType::Shrubland,
            FuelType::DeadWood,
        ]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum WeatherPresetType {
    PerthMetro,
    SouthWest,
    Wheatbelt,
    Goldfields,
    Kimberley,
    Pilbara,
    Catastrophic,
}

impl WeatherPresetType {
    fn name(self) -> &'static str {
        match self {
            WeatherPresetType::PerthMetro => "Perth Metro",
            WeatherPresetType::SouthWest => "South West",
            WeatherPresetType::Wheatbelt => "Wheatbelt",
            WeatherPresetType::Goldfields => "Goldfields",
            WeatherPresetType::Kimberley => "Kimberley",
            WeatherPresetType::Pilbara => "Pilbara",
            WeatherPresetType::Catastrophic => "Catastrophic",
        }
    }

    fn to_system(self) -> WeatherSystem {
        WeatherSystem::from_preset(
            match self {
                WeatherPresetType::PerthMetro => WeatherPreset::perth_metro(),
                WeatherPresetType::SouthWest => WeatherPreset::south_west(),
                WeatherPresetType::Wheatbelt => WeatherPreset::wheatbelt(),
                WeatherPresetType::Goldfields => WeatherPreset::goldfields(),
                WeatherPresetType::Kimberley => WeatherPreset::kimberley(),
                WeatherPresetType::Pilbara => WeatherPreset::pilbara(),
                WeatherPresetType::Catastrophic => return WeatherSystem::catastrophic(),
            },
            3,
            14.00,
            fire_sim_core::ClimatePattern::Neutral,
        )
    }

    fn all() -> [Self; 7] {
        [
            WeatherPresetType::PerthMetro,
            WeatherPresetType::SouthWest,
            WeatherPresetType::Wheatbelt,
            WeatherPresetType::Goldfields,
            WeatherPresetType::Kimberley,
            WeatherPresetType::Pilbara,
            WeatherPresetType::Catastrophic,
        ]
    }
}

// Enum that will be used as a global state for the game
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Menu,
    InGame,
}

/// Main entry point
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Australia Fire Simulation - Bevy Demo".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(MeshPickingPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EntityCountDiagnosticsPlugin::default())
        .add_plugins(SystemInformationDiagnosticsPlugin)
        .init_resource::<DemoConfig>()
        .init_state::<GameState>()
        .add_systems(Startup, setup_camera)
        // Menu state systems
        .add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(OnExit(GameState::Menu), cleanup_menu)
        .add_systems(
            EguiPrimaryContextPass,
            render_menu_ui.run_if(in_state(GameState::Menu)),
        )
        // Game state systems
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(OnExit(GameState::InGame), cleanup_game)
        .add_systems(
            Update,
            (
                update_simulation,
                update_fuel_visuals,
                update_camera_controls,
                handle_controls,
            )
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            EguiPrimaryContextPass,
            update_ui.run_if(in_state(GameState::InGame)),
        )
        .run();
}

/// Tag component for menu entities
#[derive(Component)]
struct OnMenuScreen;

/// Tag component for game entities
#[derive(Component)]
struct OnGameScreen;

/// Setup persistent camera for UI rendering (egui needs this)
fn setup_camera(mut commands: Commands) {
    // Spawn Camera2d without OnMenuScreen marker so it persists across state transitions
    // This is required for egui to work properly
    // Set order to 1 so it renders after Camera3d (order 0), placing UI on top
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
    ));
}

/// Setup menu - Camera2d stays active
fn setup_menu() {
    // Camera2d is already spawned in setup_camera at startup and stays active
    // Nothing to do here, but we keep the function for symmetry
}

/// Cleanup menu entities when exiting menu state
fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<OnMenuScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Cleanup game entities when exiting game state  
fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, With<OnGameScreen>>,
    children_query: Query<&Children>,
) {
    // Despawn all game entities (including their children recursively)
    for entity in query.iter() {
        despawn_with_children_recursive(&mut commands, entity, &children_query);
    }

    // Remove resources
    commands.remove_resource::<AmbientLight>();
    commands.remove_resource::<SimulationState>();
}

/// Recursively despawn an entity and all its children
fn despawn_with_children_recursive(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children>,
) {
    // First despawn all children recursively
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            despawn_with_children_recursive(commands, child, children_query);
        }
    }

    // Then despawn the entity itself
    commands.entity(entity).despawn();
}

/// Render egui menu UI
fn render_menu_ui(
    mut contexts: EguiContexts,
    mut config: ResMut<DemoConfig>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Make the entire menu scrollable for small windows
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading(
                    egui::RichText::new("Australia Fire Simulation")
                        .size(48.0)
                        .color(egui::Color32::from_rgb(255, 204, 51)),
                );
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Configure Simulation Parameters")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(204, 204, 204)),
                );
                ui.add_space(30.0);
            });

            // Center content with max width
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.set_max_width(700.0);

                // Terrain Settings
                ui.group(|ui| {
                    ui.heading(
                        egui::RichText::new("TERRAIN SETTINGS")
                            .size(20.0)
                            .color(egui::Color32::from_rgb(255, 204, 51)),
                    );
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Map Width (m):");
                        ui.add(egui::Slider::new(&mut config.map_width, 50.0..=500.0).text("m"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Map Height (m):");
                        ui.add(egui::Slider::new(&mut config.map_height, 50.0..=500.0).text("m"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Terrain Type:");
                        egui::ComboBox::new("terrain-type", "")
                            .selected_text(config.terrain_type.name())
                            .show_ui(ui, |ui| {
                                for variant in TerrainType::all() {
                                    ui.selectable_value(
                                        &mut config.terrain_type,
                                        variant,
                                        variant.name(),
                                    );
                                }
                            });
                    });
                });

                ui.add_space(10.0);

                // Fire Settings
                ui.group(|ui| {
                    ui.heading(
                        egui::RichText::new("FIRE SETTINGS")
                            .size(20.0)
                            .color(egui::Color32::from_rgb(255, 204, 51)),
                    );
                    ui.add_space(10.0);

                    // Guard for spacing > 0 and ensure at least 1
                    let max_x = ((config.map_width / config.spacing).floor() as usize).max(1);
                    let max_y = ((config.map_height / config.spacing).floor() as usize).max(1);

                    ui.horizontal(|ui| {
                        ui.label("Elements X:");
                        ui.add(egui::Slider::new(&mut config.elements_x, 1..=max_x));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Elements Y:");
                        ui.add(egui::Slider::new(&mut config.elements_y, 1..=max_y));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Fuel Mass (kg):");
                        ui.add(egui::Slider::new(&mut config.fuel_mass, 1.0..=20.0).text("kg"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Fuel Type:");
                        egui::ComboBox::new("fuel-type", "")
                            .selected_text(config.fuel_type.name())
                            .show_ui(ui, |ui| {
                                for variant in FuelType::all() {
                                    ui.selectable_value(
                                        &mut config.fuel_type,
                                        variant,
                                        variant.name(),
                                    );
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Spacing (m):");
                        ui.add(egui::Slider::new(&mut config.spacing, 5.0..=20.0).text("m"));
                    });
                });

                ui.add_space(10.0);

                // Weather Settings
                ui.group(|ui| {
                    ui.heading(
                        egui::RichText::new("WEATHER SETTINGS")
                            .size(20.0)
                            .color(egui::Color32::from_rgb(255, 204, 51)),
                    );
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut config.use_weather_preset, "Use Weather Preset");
                    });

                    if config.use_weather_preset {
                        ui.horizontal(|ui| {
                            ui.label("Weather Preset:");
                            egui::ComboBox::new("weather-preset", "")
                                .selected_text(config.weather_preset.name())
                                .show_ui(ui, |ui| {
                                    for variant in WeatherPresetType::all() {
                                        ui.selectable_value(
                                            &mut config.weather_preset,
                                            variant,
                                            variant.name(),
                                        );
                                    }
                                });
                        });
                        ui.label(
                            egui::RichText::new("(Dynamic weather will be simulated)")
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Temperature (°C):");
                            ui.add(
                                egui::Slider::new(&mut config.temperature, 10.0..=50.0).text("°C"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Humidity (%):");
                            ui.add(egui::Slider::new(&mut config.humidity, 1.0..=80.0).text("%"));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Wind Speed (km/h):");
                            ui.add(
                                egui::Slider::new(&mut config.wind_speed, 0.0..=40.0).text("km/h"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Wind Direction (degrees):");
                            ui.add(
                                egui::Slider::new(&mut config.wind_direction, 0.0..=360.0)
                                    .text("°"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Drought Factor:");
                            ui.add(egui::Slider::new(&mut config.drought_factor, 1.0..=20.0));
                        });
                    }
                });

                ui.add_space(30.0);

                // Buttons - now inside the scroll area
                ui.vertical_centered(|ui| {
                    if ui
                        .add_sized(
                            [300.0, 60.0],
                            egui::Button::new(egui::RichText::new("START SIMULATION").size(24.0)),
                        )
                        .clicked()
                    {
                        next_state.set(GameState::InGame);
                    }
                    ui.add_space(10.0);

                    if ui
                        .add_sized(
                            [300.0, 50.0],
                            egui::Button::new(egui::RichText::new("QUIT").size(20.0))
                                .fill(egui::Color32::from_rgb(180, 60, 60)),
                        )
                        .clicked()
                    {
                        exit.write(bevy::app::AppExit::Success);
                    }
                    ui.add_space(20.0);
                });
            });
        });
    });

    Ok(())
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
        Self::from_config(&config)
    }
}

impl SimulationState {
    fn from_config(config: &DemoConfig) -> Self {
        let terrain_hill = config.map_height.min(config.map_width) * 0.15;

        // Create terrain based on config
        let terrain = match config.terrain_type {
            TerrainType::Flat => TerrainData::flat(config.map_width, config.map_height, 5.0, 0.0),
            TerrainType::Hill => TerrainData::single_hill(
                config.map_width,
                config.map_height,
                5.0,
                0.0,
                terrain_hill,
                terrain_hill,
            ),
            TerrainType::Valley => TerrainData::valley_between_hills(
                config.map_width,
                config.map_height,
                5.0,
                0.0,
                terrain_hill,
            ),
        };

        // Create simulation
        let mut sim = FireSimulation::new(5.0, terrain);

        // Set weather conditions from config
        let weather = if config.use_weather_preset {
            // Use dynamic weather preset
            config.weather_preset.to_system()
        } else {
            // Use static weather values
            WeatherSystem::new(
                config.temperature,
                config.humidity,
                config.wind_speed,
                config.wind_direction,
                config.drought_factor,
            )
        };
        sim.set_weather(weather);

        // Add fuel elements in a grid
        let center_x = config.map_width / 2.0;
        let center_y = config.map_height / 2.0;
        let start_x =
            center_x - (config.elements_x.saturating_sub(1) as f32 * config.spacing) / 2.0;
        let start_y =
            center_y - (config.elements_y.saturating_sub(1) as f32 * config.spacing) / 2.0;

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

        // No auto-ignition - user will manually ignite fuel elements with 'I' key

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

/// Marker for terrain mesh
#[derive(Component)]
struct TerrainMesh;

/// Setup game when transitioning from menu
fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<DemoConfig>,
) {
    // Camera2d (order 1) stays active for UI overlay
    // Camera3d (order 0) renders the 3D scene first
    // Different orders prevent rendering conflicts

    // Initialize simulation state resource
    let mut sim_state = SimulationState::from_config(&config);

    // Add light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        OnGameScreen,
    ));

    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        affects_lightmapped_meshes: false,
    });

    // Add camera with position based on map size
    // Calculate camera position to frame the entire map
    let map_center_x = config.map_width / 2.0;
    let map_center_z = config.map_height / 2.0;
    let map_max_dim = config.map_width.max(config.map_height);

    // Camera distance scales with map size (larger maps need more zoom out)
    // Base distance for 200x200 map is 150, scale proportionally
    let camera_distance = (map_max_dim / 200.0) * 150.0;
    let camera_height = (map_max_dim / 200.0) * 120.0;

    // Position camera at 45-degree angle to view the map
    let camera_offset = camera_distance * 0.707; // cos(45°) ≈ 0.707
    let camera_x = map_center_x + camera_offset;
    let camera_z = map_center_z + camera_offset;

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0, // Render first (3D scene)
            ..default()
        },
        Transform::from_xyz(camera_x, camera_height, camera_z)
            .looking_at(Vec3::new(map_center_x, 0.0, map_center_z), Vec3::Y),
        MainCamera,
        OnGameScreen,
    ));

    // Create terrain mesh
    let terrain = &sim_state.simulation.grid.terrain;
    spawn_terrain(&mut commands, &mut meshes, &mut materials, terrain);

    // Spawn fuel element visuals
    let cube_mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));

    // Collect elements first to avoid borrow issues
    let elements: Vec<_> = sim_state
        .simulation
        .get_all_elements()
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

        let entity = commands
            .spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(pos.x, pos.z, pos.y),
                FuelVisual { element_id },
                OnGameScreen,
            ))
            .id();

        sim_state.fuel_entity_map.push((element_id, entity));
    }

    // Setup UI
    setup_ui(&mut commands);

    // Insert simulation state as a resource
    commands.insert_resource(sim_state);
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
    for normal in normals.iter_mut().take(positions.len()) {
        *normal = [0.0, 0.0, 0.0];
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
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    let terrain_mesh = meshes.add(mesh);
    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(terrain_mesh),
        MeshMaterial3d(terrain_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        TerrainMesh,
        Pickable::default(), // Make terrain pickable for ray casting
        OnGameScreen,
    ));
}

fn setup_ui(_commands: &mut Commands) {
    // No longer need to spawn UI entities - egui will handle it all
    // We'll keep this function for consistency but it's now empty
}

/// Update the simulation
fn update_simulation(time: Res<Time>, mut sim_state: ResMut<SimulationState>) {
    if sim_state.paused {
        return;
    }

    // Accumulate time with speed multiplier
    sim_state.time_accumulator += time.delta_secs() * sim_state.speed;

    // Update simulation at 1 FPS (1.0 second timesteps) for proper heat transfer physics
    // Smaller timesteps cause heat transfer to be too weak for realistic fire spread
    let timestep = 1.0;
    while sim_state.time_accumulator >= timestep {
        sim_state.simulation.update(timestep);
        sim_state.time_accumulator -= timestep;
    }
}

/// Update fuel element visuals based on simulation state
fn update_fuel_visuals(
    sim_state: Res<SimulationState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&FuelVisual, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (fuel_visual, material_handle) in query.iter() {
        if let Some(element) = sim_state
            .simulation
            .get_all_elements()
            .into_iter()
            .find(|e| e.id == fuel_visual.element_id)
        {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                if element.is_ignited() {
                    // Calculate color based on temperature
                    let temp_factor = (element.temperature() / 1200.0).clamp(0.0, 1.0);
                    material.base_color = Color::srgb(1.0, 0.5 - temp_factor * 0.3, 0.0);
                    material.emissive = LinearRgba::rgb(5.0 * temp_factor, 2.0 * temp_factor, 0.0);
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
) -> Result {
    let mut transform = query.single_mut()?;
    let speed = 50.0 * time.delta_secs();
    let rotation_speed = 1.0 * time.delta_secs();

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

    Ok(())
}

/// Render game UI using egui
fn update_ui(
    mut contexts: EguiContexts,
    sim_state: Res<SimulationState>,
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    fuel_query: Query<(&GlobalTransform, &FuelVisual)>,
) -> Result {
    let stats = sim_state.simulation.get_stats();
    let weather = &sim_state.simulation.weather;

    // Get FPS from diagnostics
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);

    // Get frame time in milliseconds
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|ft| ft.smoothed())
        .unwrap_or(0.0);

    // Calculate entity count for performance tracking
    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|ec| ec.value())
        .unwrap_or(0.0);

    // Get CPU usage from system information diagnostics
    let cpu_usage = diagnostics
        .get(&SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE)
        .and_then(|cpu| cpu.smoothed())
        .unwrap_or(0.0);

    // Get memory usage from system information diagnostics
    let mem_usage = diagnostics
        .get(&SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE)
        .and_then(|mem| mem.smoothed())
        .unwrap_or(0.0);

    let ctx = contexts.ctx_mut()?;

    // Title and controls panel (top-left)
    egui::Window::new("Australia Fire Simulation")
        .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.colored_label(egui::Color32::LIGHT_GRAY, "Controls:");
            ui.label("  SPACE - Pause/Resume");
            ui.label("  [ / ] - Speed Down/Up");
            ui.label("  R - Reset");
            ui.label("  I - Ignite Fuel (at cursor)");
            ui.label("  W - Add Water Suppression");
            ui.label("  ESC - Return to Menu");
            ui.label("  Arrow Keys - Camera");
            ui.label("  Hover - Element Details");
        });

    // Stats panel (right side)
    egui::Window::new("SIMULATION STATISTICS")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .default_width(400.0)
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.label(format!("Simulation Time: {:.1}s", stats.simulation_time));
            ui.label(format!(
                "Status: {}",
                if sim_state.paused {
                    "PAUSED"
                } else {
                    "RUNNING"
                }
            ));
            ui.label(format!("Speed: {:.1}x", sim_state.speed));

            ui.separator();
            ui.heading("SYSTEM PERFORMANCE");
            ui.label(format!("FPS: {:.0}", fps));
            ui.label(format!("Frame Time: {:.2}ms", frame_time));
            ui.label(format!("Entities: {:.0}", entity_count));
            ui.label(format!("Delta Time: {:.3}s", time.delta_secs()));

            ui.separator();
            ui.heading("HARDWARE PERFORMANCE");
            ui.label(format!("CPU Usage: {:.1}%", cpu_usage));
            ui.label(format!("Memory Usage: {:.1} MB", mem_usage));

            ui.separator();
            ui.heading("FIRE STATUS");
            ui.label(format!("Burning Elements: {}", stats.burning_elements));
            ui.label(format!("Total Elements: {}", stats.total_elements));
            ui.label(format!(
                "Fuel Consumed: {:.1} kg",
                stats.total_fuel_consumed
            ));
            ui.label(format!(
                "Max Temperature: {:.0}°C",
                sim_state
                    .simulation
                    .get_all_elements()
                    .iter()
                    .map(|e| e.temperature())
                    .fold(0.0f32, f32::max)
            ));

            ui.separator();
            ui.heading("WEATHER CONDITIONS");
            ui.label(format!("Temperature: {:.0}°C", weather.temperature));
            ui.label(format!("Humidity: {:.0}%", weather.humidity));
            ui.label(format!("Wind Speed: {:.1} km/h", weather.wind_speed));
            ui.label(format!("Wind Direction: {:.0}°", weather.wind_direction,));
            ui.label(format!("Drought Factor: {:.1}", weather.drought_factor));

            ui.separator();
            ui.heading("FIRE DANGER");
            ui.label(format!("FFDI: {:.1}", weather.calculate_ffdi()));
            ui.label(format!("Rating: {}", weather.fire_danger_rating()));
        });

    // Tooltip on hover
    if let Ok(window) = windows.single() {
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = camera_query.single() {
                // Convert cursor position to world ray
                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                    // Find closest fuel element to ray
                    let mut closest_element: Option<(u32, f32)> = None;

                    for (transform, fuel_visual) in fuel_query.iter() {
                        let element_pos = transform.translation();
                        let to_element = element_pos - ray.origin;
                        let projection = to_element.dot(*ray.direction);

                        if projection > 0.0 {
                            let closest_point = ray.origin + ray.direction * projection;
                            let distance = (element_pos - closest_point).length();

                            if distance < 2.0 {
                                if let Some((_, current_dist)) = closest_element {
                                    if distance < current_dist {
                                        closest_element = Some((fuel_visual.element_id, distance));
                                    }
                                } else {
                                    closest_element = Some((fuel_visual.element_id, distance));
                                }
                            }
                        }
                    }

                    // Show tooltip if we found an element
                    if let Some((element_id, _)) = closest_element {
                        if let Some(element) = sim_state
                            .simulation
                            .get_all_elements()
                            .into_iter()
                            .find(|e| e.id == element_id)
                        {
                            egui::Area::new(egui::Id::new("fuel_tooltip"))
                                .fixed_pos(
                                    ctx.pointer_latest_pos().unwrap_or_default()
                                        + egui::vec2(15.0, 15.0),
                                )
                                .show(ctx, |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        ui.label(format!(
                                            "Fuel Element #{} ({})",
                                            element.id, element.fuel.name
                                        ));
                                        ui.label(format!(
                                            "Position: ({:.1}, {:.1}, {:.1})",
                                            element.position.x,
                                            element.position.y,
                                            element.position.z
                                        ));
                                        ui.label(format!(
                                            "Temperature: {:.0}°C",
                                            element.temperature()
                                        ));
                                        ui.label(format!(
                                            "Fuel Remaining: {:.2} kg",
                                            element.fuel_remaining()
                                        ));
                                        ui.label(format!(
                                            "Moisture: {:.1}%",
                                            element.moisture_fraction() * 100.0
                                        ));
                                        ui.label(format!(
                                            "Status: {}",
                                            if element.is_ignited() {
                                                "BURNING"
                                            } else if element.fuel_remaining() < 0.1 {
                                                "CONSUMED"
                                            } else {
                                                "Unburned"
                                            }
                                        ));
                                        ui.label(format!(
                                            "Flame Height: {:.2} m",
                                            element.flame_height()
                                        ));
                                    });
                                });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// Control's require lots of args.
#[allow(clippy::too_many_arguments)]
/// Handle user controls
fn handle_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sim_state: ResMut<SimulationState>,
    config: Res<DemoConfig>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut ray_cast: MeshRayCast,
) {
    // Reset simulation - recreate from config
    if keyboard.just_pressed(KeyCode::KeyR) {
        *sim_state = SimulationState::from_config(&config);
    }

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

    // Manual ignition at cursor position
    if keyboard.just_pressed(KeyCode::KeyI) {
        // Try to get cursor position and convert to world coordinates
        if let Ok(window) = windows.single() {
            if let Some(cursor_position) = window.cursor_position() {
                println!(
                    "DEBUG: Cursor at ({:.2}, {:.2}), window size: {:.2}x{:.2}",
                    cursor_position.x,
                    cursor_position.y,
                    window.width(),
                    window.height()
                );

                // Validate cursor is within window bounds
                if cursor_position.x >= 0.0
                    && cursor_position.y >= 0.0
                    && cursor_position.x <= window.width()
                    && cursor_position.y <= window.height()
                {
                    if let Ok((camera, camera_transform)) = camera_query.single() {
                        // Cast ray from camera through cursor position
                        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position)
                        {
                            // Use bevy_picking's MeshRayCast to find terrain intersection
                            let settings = MeshRayCastSettings::default().always_early_exit(); // Stop at first hit

                            let hits = ray_cast.cast_ray(ray, &settings);

                            if let Some((_entity, hit)) = hits.first() {
                                // hit.point is in world space (Bevy coordinates: x, y=height, z)
                                let world_pos = hit.point;
                                println!(
                                    "Terrain hit at world position: ({:.2}, {:.2}, {:.2})",
                                    world_pos.x, world_pos.y, world_pos.z
                                );

                                // Convert Bevy world coords to sim coords
                                // Bevy: (x, y=height, z) -> Sim: (x, z, y=height)
                                let ignite_x = world_pos.x;
                                let ignite_y = world_pos.z;

                                println!(
                                    "Cursor world position (Sim X,Y): ({:.2}, {:.2})",
                                    ignite_x, ignite_y
                                );

                                // Find the closest fuel element to cursor position and ignite it
                                let elements = sim_state.simulation.get_all_elements();
                                let mut closest_id: Option<u32> = None;
                                let mut closest_dist = f32::MAX;

                                for element in &elements {
                                    // element.position is sim coords (x, y)
                                    let dx = element.position.x - ignite_x;
                                    let dy = element.position.y - ignite_y;
                                    let dist = (dx * dx + dy * dy).sqrt();

                                    if dist < closest_dist && dist < 10.0 {
                                        // Within 10m
                                        closest_dist = dist;
                                        closest_id = Some(element.id);
                                    }
                                }

                                // Ignite the closest element
                                if let Some(id) = closest_id {
                                    println!(
                                        "Igniting element {} at distance {:.2}m from cursor",
                                        id, closest_dist
                                    );
                                    sim_state.simulation.ignite_element(id, 600.0);
                                } else {
                                    println!(
                                        "No fuel element found within 10m of cursor position (Sim X,Y): ({:.2}, {:.2})",
                                        ignite_x, ignite_y
                                    );
                                    println!("Total fuel elements: {}", elements.len());
                                    if let Some(first) = elements.first() {
                                        println!(
                                            "First element sim position (x,y): ({:.2}, {:.2})",
                                            first.position.x, first.position.y
                                        );
                                    }
                                }
                            } else {
                                println!("DEBUG: No terrain intersection found");
                            }
                        } else {
                            println!("DEBUG: Failed to cast ray from camera");
                        }
                    } else {
                        println!("DEBUG: Camera not available");
                    }
                } else {
                    println!("DEBUG: Cursor outside window bounds");
                }
            } else {
                println!("DEBUG: No cursor position available");
            }
        }
    }

    // Add water suppression at cursor position
    if keyboard.just_pressed(KeyCode::KeyW) {
        // Try to get cursor position and convert to world coordinates
        if let Ok(window) = windows.single() {
            if let Some(cursor_position) = window.cursor_position() {
                // Validate cursor is within window bounds
                if cursor_position.x >= 0.0
                    && cursor_position.y >= 0.0
                    && cursor_position.x <= window.width()
                    && cursor_position.y <= window.height()
                {
                    if let Ok((camera, camera_transform)) = camera_query.single() {
                        // Cast ray from camera through cursor position
                        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position)
                        {
                            // Use bevy_picking's MeshRayCast to find terrain intersection
                            let settings = MeshRayCastSettings::default().always_early_exit(); // Stop at first hit

                            let hits = ray_cast.cast_ray(ray, &settings);

                            if let Some((_entity, hit)) = hits.first() {
                                // hit.point is in world space (Bevy coordinates: x, y=height, z)
                                let world_pos = hit.point;

                                // Convert Bevy world coords to sim coords
                                // Bevy: (x, y=height, z) -> Sim: (x, z, y=height)
                                let drop_x = world_pos.x;
                                let drop_y = world_pos.z;
                                let terrain_elevation = world_pos.y;

                                println!(
                                    "Applying water directly at ({:.2}, {:.2}, {:.2}) - 300 kg total",
                                    drop_x, drop_y, terrain_elevation
                                );

                                // Apply water directly in a circular pattern at cursor position
                                // Distribute 300 kg total over 30 points (10 kg each)
                                for i in 0..30 {
                                    let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
                                    let radius = 25.0;
                                    let apply_x = drop_x + angle.cos() * radius;
                                    let apply_y = drop_y + angle.sin() * radius;

                                    // Apply directly at ground level
                                    sim_state.simulation.apply_suppression_direct(
                                        SimVec3::new(apply_x, apply_y, terrain_elevation),
                                        10.0,
                                        SuppressionAgent::Water,
                                    );
                                }
                            } else {
                                println!("DEBUG: No terrain intersection found for water drop");
                            }
                        }
                    }
                }
            }
        }
    }

    // Return to main menu
    if keyboard.just_pressed(KeyCode::Escape) {
        // Transition back to menu state - OnExit will handle cleanup automatically
        next_state.set(GameState::Menu);
    }
}
