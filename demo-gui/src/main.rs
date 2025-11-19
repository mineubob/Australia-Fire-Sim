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
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Australia Fire Simulation - Bevy Demo".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<DemoConfig>()
        .init_resource::<MenuState>()
        .init_resource::<FpsCounter>()
        .add_systems(Startup, setup_menu)
        .add_systems(Update, (
            handle_menu_interactions,
            update_config_display,
        ).run_if(in_menu))
        .add_systems(Update, (
            init_simulation_system.run_if(should_start_simulation),
            setup_scene.run_if(should_setup_scene),
            update_simulation.run_if(simulation_running),
            update_fuel_visuals.run_if(simulation_running),
            update_camera_controls.run_if(simulation_running),
            update_ui.run_if(simulation_running),
            handle_controls.run_if(simulation_running),
            update_tooltip.run_if(simulation_running),
            update_fps.run_if(simulation_running),
        ))
        .run();
}

/// FPS counter resource
#[derive(Resource)]
struct FpsCounter {
    frame_count: u32,
    timer: f32,
    fps: f32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            frame_count: 0,
            timer: 0.0,
            fps: 60.0,
        }
    }
}

/// Menu state resource
#[derive(Resource)]
struct MenuState {
    in_menu: bool,
    simulation_initialized: bool,
    scene_setup: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            in_menu: true,
            simulation_initialized: false,
            scene_setup: false,
        }
    }
}

// Run conditions
fn in_menu(menu_state: Res<MenuState>) -> bool {
    menu_state.in_menu
}

fn should_start_simulation(menu_state: Res<MenuState>) -> bool {
    !menu_state.in_menu && !menu_state.simulation_initialized
}

fn should_setup_scene(menu_state: Res<MenuState>) -> bool {
    !menu_state.in_menu && menu_state.simulation_initialized && !menu_state.scene_setup
}

fn simulation_running(menu_state: Res<MenuState>) -> bool {
    !menu_state.in_menu && menu_state.simulation_initialized && menu_state.scene_setup
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

#[derive(Component)]
struct TooltipText;

#[derive(Component)]
struct StatsPanel;

#[derive(Component)]
struct FpsText;

/// Menu components
#[derive(Component)]
struct MenuUI;

#[derive(Component)]
struct StartButton;

#[derive(Component, Clone, Copy)]
enum ConfigButton {
    IncrementMapWidth,
    DecrementMapWidth,
    IncrementMapHeight,
    DecrementMapHeight,
    IncrementElementsX,
    DecrementElementsX,
    IncrementElementsY,
    DecrementElementsY,
    IncrementFuelMass,
    DecrementFuelMass,
    IncrementInitialIgnitions,
    DecrementInitialIgnitions,
    IncrementSpacing,
    DecrementSpacing,
    IncrementTemperature,
    DecrementTemperature,
    IncrementHumidity,
    DecrementHumidity,
    IncrementWindSpeed,
    DecrementWindSpeed,
    IncrementWindDirection,
    DecrementWindDirection,
    IncrementDroughtFactor,
    DecrementDroughtFactor,
    CycleTerrainType,
    CycleFuelType,
}

#[derive(Component)]
struct ConfigValueText(ConfigButton);

/// Setup menu UI
fn setup_menu(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            ..default()
        },
        MenuUI,
    ))
    .with_children(|parent| {
        // Title
        parent.spawn(TextBundle::from_section(
            "Australia Fire Simulation",
            TextStyle {
                font_size: 48.0,
                color: Color::srgb(1.0, 0.8, 0.2),
                ..default()
            },
        ));
        
        parent.spawn(TextBundle::from_section(
            "Configure Simulation Parameters",
            TextStyle {
                font_size: 24.0,
                color: Color::srgb(0.8, 0.8, 0.8),
                ..default()
            },
        ).with_style(Style {
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(10.0), Val::Px(30.0)),
            ..default()
        }));
        
        // Scrollable config panel
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(10.0),
                max_width: Val::Px(700.0),
                max_height: Val::Px(400.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
            ..default()
        })
        .with_children(|config_panel| {
            add_config_section(config_panel, "TERRAIN SETTINGS");
            add_numeric_config(config_panel, "Map Width (m):", ConfigButton::DecrementMapWidth, ConfigButton::IncrementMapWidth);
            add_numeric_config(config_panel, "Map Height (m):", ConfigButton::DecrementMapHeight, ConfigButton::IncrementMapHeight);
            add_cycle_config(config_panel, "Terrain Type:", ConfigButton::CycleTerrainType);
            
            add_config_section(config_panel, "FIRE SETTINGS");
            add_numeric_config(config_panel, "Grid Width:", ConfigButton::DecrementElementsX, ConfigButton::IncrementElementsX);
            add_numeric_config(config_panel, "Grid Height:", ConfigButton::DecrementElementsY, ConfigButton::IncrementElementsY);
            add_numeric_config(config_panel, "Fuel Mass (kg):", ConfigButton::DecrementFuelMass, ConfigButton::IncrementFuelMass);
            add_cycle_config(config_panel, "Fuel Type:", ConfigButton::CycleFuelType);
            add_numeric_config(config_panel, "Initial Ignitions:", ConfigButton::DecrementInitialIgnitions, ConfigButton::IncrementInitialIgnitions);
            add_numeric_config(config_panel, "Spacing (m):", ConfigButton::DecrementSpacing, ConfigButton::IncrementSpacing);
            
            add_config_section(config_panel, "WEATHER SETTINGS");
            add_numeric_config(config_panel, "Temperature (°C):", ConfigButton::DecrementTemperature, ConfigButton::IncrementTemperature);
            add_numeric_config(config_panel, "Humidity (0-1):", ConfigButton::DecrementHumidity, ConfigButton::IncrementHumidity);
            add_numeric_config(config_panel, "Wind Speed (m/s):", ConfigButton::DecrementWindSpeed, ConfigButton::IncrementWindSpeed);
            add_numeric_config(config_panel, "Wind Direction (°):", ConfigButton::DecrementWindDirection, ConfigButton::IncrementWindDirection);
            add_numeric_config(config_panel, "Drought Factor:", ConfigButton::DecrementDroughtFactor, ConfigButton::IncrementDroughtFactor);
        });
        
        // START button
        parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(300.0),
                    height: Val::Px(60.0),
                    margin: UiRect::top(Val::Px(30.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                ..default()
            },
            StartButton,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                "START SIMULATION",
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
    });
}

fn add_config_section(parent: &mut ChildBuilder, title: &str) {
    parent.spawn(TextBundle::from_section(
        title,
        TextStyle {
            font_size: 20.0,
            color: Color::srgb(1.0, 0.8, 0.2),
            ..default()
        },
    ).with_style(Style {
        margin: UiRect::top(Val::Px(10.0)),
        ..default()
    }));
}

fn add_numeric_config(parent: &mut ChildBuilder, label: &str, dec_button: ConfigButton, inc_button: ConfigButton) {
    parent.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        },
        ..default()
    })
    .with_children(|row| {
        row.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font_size: 16.0,
                color: Color::srgb(0.9, 0.9, 0.9),
                ..default()
            },
        ));
        
        row.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|controls| {
            // Decrement button
            controls.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                    ..default()
                },
                dec_button,
            ))
            .with_children(|btn| {
                btn.spawn(TextBundle::from_section(
                    "-",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            });
            
            // Value display
            controls.spawn((
                TextBundle::from_section(
                    "0",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.7, 0.9, 1.0),
                        ..default()
                    },
                ),
                ConfigValueText(inc_button),
            ));
            
            // Increment button
            controls.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                    ..default()
                },
                inc_button,
            ))
            .with_children(|btn| {
                btn.spawn(TextBundle::from_section(
                    "+",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            });
        });
    });
}

fn add_cycle_config(parent: &mut ChildBuilder, label: &str, cycle_button: ConfigButton) {
    parent.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        },
        ..default()
    })
    .with_children(|row| {
        row.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font_size: 16.0,
                color: Color::srgb(0.9, 0.9, 0.9),
                ..default()
            },
        ));
        
        row.spawn((
            ButtonBundle {
                style: Style {
                    padding: UiRect::all(Val::Px(8.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                ..default()
            },
            cycle_button,
        ))
        .with_children(|btn| {
            btn.spawn((
                TextBundle::from_section(
                    "Value",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.7, 0.9, 1.0),
                        ..default()
                    },
                ),
                ConfigValueText(cycle_button),
            ));
        });
    });
}

/// Handle menu button interactions
fn handle_menu_interactions(
    mut config: ResMut<DemoConfig>,
    mut menu_state: ResMut<MenuState>,
    mut interaction_query: Query<
        (&Interaction, Option<&ConfigButton>, Option<&StartButton>, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    menu_query: Query<Entity, With<MenuUI>>,
    mut commands: Commands,
) {
    for (interaction, config_button, start_button, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if start_button.is_some() {
                    // Hide menu
                    for entity in menu_query.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                    menu_state.in_menu = false;
                } else if let Some(button) = config_button {
                    // Update config based on button
                    match button {
                        ConfigButton::IncrementMapWidth => config.map_width += 10.0,
                        ConfigButton::DecrementMapWidth => config.map_width = (config.map_width - 10.0).max(50.0),
                        ConfigButton::IncrementMapHeight => config.map_height += 10.0,
                        ConfigButton::DecrementMapHeight => config.map_height = (config.map_height - 10.0).max(50.0),
                        ConfigButton::IncrementElementsX => config.elements_x += 1,
                        ConfigButton::DecrementElementsX => config.elements_x = config.elements_x.saturating_sub(1).max(1),
                        ConfigButton::IncrementElementsY => config.elements_y += 1,
                        ConfigButton::DecrementElementsY => config.elements_y = config.elements_y.saturating_sub(1).max(1),
                        ConfigButton::IncrementFuelMass => config.fuel_mass += 0.5,
                        ConfigButton::DecrementFuelMass => config.fuel_mass = (config.fuel_mass - 0.5).max(0.5),
                        ConfigButton::IncrementInitialIgnitions => config.initial_ignitions += 1,
                        ConfigButton::DecrementInitialIgnitions => config.initial_ignitions = config.initial_ignitions.saturating_sub(1).max(1),
                        ConfigButton::IncrementSpacing => config.spacing += 0.5,
                        ConfigButton::DecrementSpacing => config.spacing = (config.spacing - 0.5).max(1.0),
                        ConfigButton::IncrementTemperature => config.temperature += 1.0,
                        ConfigButton::DecrementTemperature => config.temperature = (config.temperature - 1.0).max(0.0),
                        ConfigButton::IncrementHumidity => config.humidity = (config.humidity + 0.01).min(1.0),
                        ConfigButton::DecrementHumidity => config.humidity = (config.humidity - 0.01).max(0.0),
                        ConfigButton::IncrementWindSpeed => config.wind_speed += 1.0,
                        ConfigButton::DecrementWindSpeed => config.wind_speed = (config.wind_speed - 1.0).max(0.0),
                        ConfigButton::IncrementWindDirection => config.wind_direction = (config.wind_direction + 15.0) % 360.0,
                        ConfigButton::DecrementWindDirection => config.wind_direction = (config.wind_direction - 15.0 + 360.0) % 360.0,
                        ConfigButton::IncrementDroughtFactor => config.drought_factor = (config.drought_factor + 0.5).min(20.0),
                        ConfigButton::DecrementDroughtFactor => config.drought_factor = (config.drought_factor - 0.5).max(0.0),
                        ConfigButton::CycleTerrainType => {
                            config.terrain_type = match config.terrain_type {
                                TerrainType::Flat => TerrainType::Hill,
                                TerrainType::Hill => TerrainType::Valley,
                                TerrainType::Valley => TerrainType::Flat,
                            };
                        }
                        ConfigButton::CycleFuelType => {
                            config.fuel_type = match config.fuel_type {
                                FuelType::DryGrass => FuelType::EucalyptusStringybark,
                                FuelType::EucalyptusStringybark => FuelType::EucalyptusSmoothBark,
                                FuelType::EucalyptusSmoothBark => FuelType::Shrubland,
                                FuelType::Shrubland => FuelType::DeadWood,
                                FuelType::DeadWood => FuelType::DryGrass,
                            };
                        }
                    }
                }
            }
            Interaction::Hovered => {
                if start_button.is_some() {
                    *color = BackgroundColor(Color::srgb(0.25, 0.7, 0.25));
                } else {
                    *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.5));
                }
            }
            Interaction::None => {
                if start_button.is_some() {
                    *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                } else {
                    *color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));
                }
            }
        }
    }
}

/// Update config value displays in menu
fn update_config_display(
    config: Res<DemoConfig>,
    mut query: Query<(&mut Text, &ConfigValueText)>,
) {
    for (mut text, config_text) in query.iter_mut() {
        text.sections[0].value = match config_text.0 {
            ConfigButton::IncrementMapWidth | ConfigButton::DecrementMapWidth => 
                format!("{:.0}", config.map_width),
            ConfigButton::IncrementMapHeight | ConfigButton::DecrementMapHeight => 
                format!("{:.0}", config.map_height),
            ConfigButton::IncrementElementsX | ConfigButton::DecrementElementsX => 
                format!("{}", config.elements_x),
            ConfigButton::IncrementElementsY | ConfigButton::DecrementElementsY => 
                format!("{}", config.elements_y),
            ConfigButton::IncrementFuelMass | ConfigButton::DecrementFuelMass => 
                format!("{:.1}", config.fuel_mass),
            ConfigButton::IncrementInitialIgnitions | ConfigButton::DecrementInitialIgnitions => 
                format!("{}", config.initial_ignitions),
            ConfigButton::IncrementSpacing | ConfigButton::DecrementSpacing => 
                format!("{:.1}", config.spacing),
            ConfigButton::IncrementTemperature | ConfigButton::DecrementTemperature => 
                format!("{:.0}", config.temperature),
            ConfigButton::IncrementHumidity | ConfigButton::DecrementHumidity => 
                format!("{:.2}", config.humidity),
            ConfigButton::IncrementWindSpeed | ConfigButton::DecrementWindSpeed => 
                format!("{:.0}", config.wind_speed),
            ConfigButton::IncrementWindDirection | ConfigButton::DecrementWindDirection => 
                format!("{:.0}", config.wind_direction),
            ConfigButton::IncrementDroughtFactor | ConfigButton::DecrementDroughtFactor => 
                format!("{:.1}", config.drought_factor),
            ConfigButton::CycleTerrainType => match config.terrain_type {
                TerrainType::Flat => "Flat".to_string(),
                TerrainType::Hill => "Hill".to_string(),
                TerrainType::Valley => "Valley".to_string(),
            },
            ConfigButton::CycleFuelType => match config.fuel_type {
                FuelType::DryGrass => "Dry Grass".to_string(),
                FuelType::EucalyptusStringybark => "Eucalyptus Stringybark".to_string(),
                FuelType::EucalyptusSmoothBark => "Eucalyptus Smooth Bark".to_string(),
                FuelType::Shrubland => "Shrubland".to_string(),
                FuelType::DeadWood => "Dead Wood".to_string(),
            },
        };
    }
}

/// Initialize simulation when transitioning from menu
fn init_simulation_system(
    mut commands: Commands,
    mut menu_state: ResMut<MenuState>,
) {
    // Initialize simulation state
    commands.init_resource::<SimulationState>();
    menu_state.simulation_initialized = true;
}

/// Setup the 3D scene after simulation is initialized
fn setup_scene(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    sim_state: ResMut<SimulationState>,
    mut menu_state: ResMut<MenuState>,
) {
    setup(commands, meshes, materials, sim_state);
    menu_state.scene_setup = true;
}

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
    // Root UI container - fills entire screen
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Left side - Title and controls
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|left| {
                // Title
                left.spawn(TextBundle::from_section(
                    "Australia Fire Simulation",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
                
                // Controls text
                left.spawn((
                    TextBundle::from_section(
                        "Controls:\n  SPACE - Pause/Resume\n  [ / ] - Speed Down/Up\n  R - Reset\n  W - Add Water Suppression\n  Arrow Keys - Camera\n  Hover - Element Details",
                        TextStyle {
                            font_size: 14.0,
                            color: Color::srgb(0.6, 0.6, 0.6),
                            ..default()
                        },
                    ),
                    ControlsText,
                ));
            });
            
            // Right side - Stats panel with background box
            parent.spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(15.0)),
                        margin: UiRect::all(Val::Px(10.0)),
                        width: Val::Px(400.0),
                        max_height: Val::Percent(90.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                    ..default()
                },
                StatsPanel,
            ))
            .with_children(|panel| {
                // Stats heading
                panel.spawn(TextBundle::from_section(
                    "SIMULATION STATISTICS",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::srgb(1.0, 0.8, 0.2),
                        ..default()
                    },
                ));
                
                // Stats text
                panel.spawn((
                    TextBundle::from_section(
                        "Initializing...",
                        TextStyle {
                            font_size: 16.0,
                            color: Color::srgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ),
                    StatsText,
                ));
            });
        });
    
    // Tooltip (hidden by default)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            text: Text::from_section(
                "",
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
            visibility: Visibility::Hidden,
            ..default()
        },
        TooltipText,
    ));
    
    // FPS counter (top-right corner)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            text: Text::from_section(
                "FPS: 60",
                TextStyle {
                    font_size: 16.0,
                    color: Color::srgb(1.0, 1.0, 0.0),
                    ..default()
                },
            ),
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ..default()
        },
        FpsText,
    ));
}

/// Update FPS counter
fn update_fps(
    time: Res<Time>,
    mut fps_counter: ResMut<FpsCounter>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    fps_counter.frame_count += 1;
    fps_counter.timer += time.delta_seconds();
    
    if fps_counter.timer >= 1.0 {
        fps_counter.fps = fps_counter.frame_count as f32 / fps_counter.timer;
        fps_counter.frame_count = 0;
        fps_counter.timer = 0.0;
        
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("FPS: {:.0}", fps_counter.fps);
        }
    }
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
            "Simulation Time: {:.1}s\n\
             Status: {}\n\
             Speed: {:.1}x\n\
             \n\
             FIRE STATUS\n\
             Burning Elements: {}\n\
             Total Elements: {}\n\
             Fuel Consumed: {:.1} kg\n\
             Max Temperature: {:.0}°C\n\
             \n\
             WEATHER CONDITIONS\n\
             Temperature: {:.0}°C\n\
             Humidity: {:.0}%\n\
             Wind Speed: {:.1} m/s\n\
             Wind Direction: {:.0}°\n\
             Drought Factor: {:.1}\n\
             \n\
             FIRE DANGER\n\
             FFDI: {:.1}\n\
             Rating: {}",
            stats.simulation_time,
            if sim_state.paused { "PAUSED" } else { "RUNNING" },
            sim_state.speed,
            stats.burning_elements,
            stats.total_elements,
            stats.total_fuel_consumed,
            sim_state.simulation.get_all_elements().iter()
                .map(|e| e.temperature())
                .fold(0.0f32, f32::max),
            weather.temperature,
            weather.humidity * 100.0,
            weather.wind_speed,
            weather.wind_direction,
            weather.drought_factor,
            weather.calculate_ffdi(),
            weather.fire_danger_rating(),
        );
    }
}

/// Update tooltip on hover
fn update_tooltip(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    fuel_query: Query<(&GlobalTransform, &FuelVisual)>,
    sim_state: Res<SimulationState>,
    mut tooltip_query: Query<(&mut Text, &mut Style, &mut Visibility), With<TooltipText>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    
    let Some(cursor_position) = window.cursor_position() else {
        // Hide tooltip if cursor not in window
        for (_, _, mut visibility) in tooltip_query.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    
    // Cast ray from camera through cursor position
    let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };
    
    // Find closest fuel element intersecting with ray
    let mut closest_element: Option<(u32, f32)> = None;
    
    for (transform, fuel_visual) in fuel_query.iter() {
        let element_pos = transform.translation();
        
        // Simple sphere collision (2m radius for the cube)
        let to_element = element_pos - ray.origin;
        let ray_dir = *ray.direction; // Convert Dir3 to Vec3
        let projection = to_element.dot(ray_dir);
        
        if projection > 0.0 {
            let closest_point = ray.origin + ray_dir * projection;
            let distance_to_ray = (element_pos - closest_point).length();
            
            if distance_to_ray < 2.0 { // Cube is 2x2x2
                let distance = projection;
                if closest_element.is_none() || distance < closest_element.unwrap().1 {
                    closest_element = Some((fuel_visual.element_id, distance));
                }
            }
        }
    }
    
    // Update tooltip
    for (mut text, mut style, mut visibility) in tooltip_query.iter_mut() {
        if let Some((element_id, _)) = closest_element {
            // Find the element in simulation
            if let Some(element) = sim_state.simulation.get_all_elements()
                .into_iter()
                .find(|e| e.id == element_id)
            {
                // Show tooltip with element details
                text.sections[0].value = format!(
                    "Fuel Element #{}\n\
                     Position: ({:.1}, {:.1}, {:.1})\n\
                     Temperature: {:.0}°C\n\
                     Fuel Remaining: {:.2} kg\n\
                     Moisture: {:.1}%\n\
                     Status: {}\n\
                     Flame Height: {:.2} m",
                    element.id,
                    element.position.x,
                    element.position.y,
                    element.position.z,
                    element.temperature(),
                    element.fuel_remaining(),
                    element.moisture_fraction() * 100.0,
                    if element.is_ignited() { "BURNING" } else if element.fuel_remaining() < 0.1 { "CONSUMED" } else { "Unburned" },
                    element.flame_height(),
                );
                
                // Position tooltip near cursor
                style.left = Val::Px(cursor_position.x + 15.0);
                style.top = Val::Px(cursor_position.y + 15.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
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
