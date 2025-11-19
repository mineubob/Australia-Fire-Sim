//! Bevy 0.17 + bevy-egui GUI demo for the Australia Fire Simulation
//!
//! This demo provides a real-time 3D visualization of the fire simulation with interactive controls.
//! Uses bevy-egui for a professional menu interface.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
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

impl TerrainType {
    fn name(&self) -> &str {
        match self {
            TerrainType::Flat => "Flat",
            TerrainType::Hill => "Hill",
            TerrainType::Valley => "Valley",
        }
    }
    
    fn cycle(&self) -> Self {
        match self {
            TerrainType::Flat => TerrainType::Hill,
            TerrainType::Hill => TerrainType::Valley,
            TerrainType::Valley => TerrainType::Flat,
        }
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
    fn name(&self) -> &str {
        match self {
            FuelType::DryGrass => "Dry Grass",
            FuelType::EucalyptusStringybark => "Eucalyptus Stringybark",
            FuelType::EucalyptusSmoothBark => "Eucalyptus Smooth Bark",
            FuelType::Shrubland => "Shrubland",
            FuelType::DeadWood => "Dead Wood",
        }
    }
    
    fn cycle(&self) -> Self {
        match self {
            FuelType::DryGrass => FuelType::EucalyptusStringybark,
            FuelType::EucalyptusStringybark => FuelType::EucalyptusSmoothBark,
            FuelType::EucalyptusSmoothBark => FuelType::Shrubland,
            FuelType::Shrubland => FuelType::DeadWood,
            FuelType::DeadWood => FuelType::DryGrass,
        }
    }
    
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
                title: "Australia Fire Simulation - Bevy 0.17 + egui Demo".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .init_resource::<DemoConfig>()
        .init_resource::<MenuState>()
        .init_resource::<FpsCounter>()
        .add_systems(Startup, setup_empty_scene)
        .add_systems(Update, (
            render_menu_ui.run_if(in_menu),
            init_simulation_system.run_if(should_start_simulation),
            setup_scene.run_if(should_setup_scene),
            update_fps.run_if(simulation_running),
            update_simulation.run_if(simulation_running),
            update_fuel_visuals.run_if(simulation_running),
            update_camera_controls.run_if(simulation_running),
            update_ui.run_if(simulation_running),
            update_tooltip.run_if(simulation_running),
            handle_controls.run_if(simulation_running),
        ))
        .run();
}

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

#[derive(Resource)]
struct MenuState {
    show_menu: bool,
    start_simulation: bool,
    setup_scene_next_frame: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            show_menu: true,
            start_simulation: false,
            setup_scene_next_frame: false,
        }
    }
}

// Run conditions
fn in_menu(menu_state: Res<MenuState>) -> bool {
    menu_state.show_menu
}

fn should_start_simulation(menu_state: Res<MenuState>) -> bool {
    menu_state.start_simulation
}

fn should_setup_scene(menu_state: Res<MenuState>) -> bool {
    menu_state.setup_scene_next_frame
}

fn simulation_running(menu_state: Res<MenuState>) -> bool {
    !menu_state.show_menu
}

// Simulation state
#[derive(Resource)]
struct SimulationState {
    simulation: FireSimulation,
    paused: bool,
    sim_time: f32,
    sim_speed: f32,
}

impl FromWorld for SimulationState {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<DemoConfig>().clone();
        
        // Create terrain
        let terrain = match config.terrain_type {
            TerrainType::Flat => TerrainData::flat(config.map_width, config.map_height),
            TerrainType::Hill => TerrainData::hill(config.map_width, config.map_height, 30.0),
            TerrainType::Valley => TerrainData::valley(config.map_width, config.map_height, 20.0),
        };
        
        // Create simulation
        let mut sim = FireSimulation::new(config.map_width, config.map_height, 50.0, terrain);
        
        // Configure weather
        sim.weather_mut().temperature = config.temperature;
        sim.weather_mut().humidity = config.humidity;
        sim.weather_mut().wind_speed = config.wind_speed;
        let wind_rad = config.wind_direction.to_radians();
        sim.weather_mut().wind_direction = SimVec3::new(wind_rad.cos(), wind_rad.sin(), 0.0);
        sim.weather_mut().drought_factor = config.drought_factor;
        
        // Add fuel elements in grid
        let fuel = config.fuel_type.to_fuel();
        let half_elements_x = config.elements_x as f32 / 2.0;
        let half_elements_y = config.elements_y as f32 / 2.0;
        
        for ix in 0..config.elements_x {
            for iy in 0..config.elements_y {
                let x = (ix as f32 - half_elements_x) * config.spacing;
                let y = (iy as f32 - half_elements_y) * config.spacing;
                let z = sim.get_terrain().get_elevation(x, y);
                
                sim.add_fuel_element(
                    SimVec3::new(x, y, z),
                    fuel.clone(),
                    config.fuel_mass,
                    FuelPart::Surface,
                    None,
                );
            }
        }
        
        // Ignite initial elements
        let element_ids: Vec<u32> = sim.get_all_elements().iter().map(|e| e.id()).collect();
        let total_elements = element_ids.len();
        let ignition_interval = if config.initial_ignitions > 0 {
            total_elements / config.initial_ignitions.min(total_elements)
        } else {
            total_elements
        };
        
        for i in 0..config.initial_ignitions {
            let idx = (i * ignition_interval).min(total_elements - 1);
            if let Some(&id) = element_ids.get(idx) {
                sim.ignite_element(id);
            }
        }
        
        Self {
            simulation: sim,
            paused: false,
            sim_time: 0.0,
            sim_speed: 1.0,
        }
    }
}

// Component markers
#[derive(Component)]
struct FuelVisual {
    element_id: u32,
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct TooltipText;

// Setup empty scene (just for egui to work initially)
fn setup_empty_scene(mut commands: Commands) {
    // Spawn a 2D camera for egui
    commands.spawn(Camera2d);
}

// Render menu UI using bevy-egui
fn render_menu_ui(
    mut contexts: EguiContexts,
    mut config: ResMut<DemoConfig>,
    mut menu_state: ResMut<MenuState>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.heading("Australia Fire Simulation - Configuration");
        ui.add_space(20.0);
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Terrain Settings
            ui.group(|ui| {
                ui.heading("Terrain Settings");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Map Width:");
                    ui.add(egui::Slider::new(&mut config.map_width, 50.0..=500.0).suffix(" m"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Map Height:");
                    ui.add(egui::Slider::new(&mut config.map_height, 50.0..=500.0).suffix(" m"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Terrain Type:");
                    if ui.button(config.terrain_type.name()).clicked() {
                        config.terrain_type = config.terrain_type.cycle();
                    }
                });
            });
            
            ui.add_space(10.0);
            
            // Fire Settings
            ui.group(|ui| {
                ui.heading("Fire Settings");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Grid Width:");
                    ui.add(egui::Slider::new(&mut config.elements_x, 5..=20));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Grid Height:");
                    ui.add(egui::Slider::new(&mut config.elements_y, 5..=20));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Fuel Mass:");
                    ui.add(egui::Slider::new(&mut config.fuel_mass, 1.0..=20.0).suffix(" kg"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Fuel Type:");
                    if ui.button(config.fuel_type.name()).clicked() {
                        config.fuel_type = config.fuel_type.cycle();
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Initial Ignitions:");
                    ui.add(egui::Slider::new(&mut config.initial_ignitions, 1..=20));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Element Spacing:");
                    ui.add(egui::Slider::new(&mut config.spacing, 5.0..=15.0).suffix(" m"));
                });
            });
            
            ui.add_space(10.0);
            
            // Weather Settings
            ui.group(|ui| {
                ui.heading("Weather Settings");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Temperature:");
                    ui.add(egui::Slider::new(&mut config.temperature, 10.0..=50.0).suffix(" °C"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Humidity:");
                    ui.add(egui::Slider::new(&mut config.humidity, 0.05..=0.60).suffix(" %"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Wind Speed:");
                    ui.add(egui::Slider::new(&mut config.wind_speed, 0.0..=40.0).suffix(" km/h"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Wind Direction:");
                    ui.add(egui::Slider::new(&mut config.wind_direction, 0.0..=360.0).suffix(" °"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Drought Factor:");
                    ui.add(egui::Slider::new(&mut config.drought_factor, 1.0..=20.0));
                });
            });
            
            ui.add_space(20.0);
            
            // Start button
            if ui.add(egui::Button::new("START SIMULATION").min_size(egui::vec2(200.0, 40.0))).clicked() {
                menu_state.start_simulation = true;
            }
        });
    });
}

// Initialize simulation
fn init_simulation_system(
    mut commands: Commands,
    config: Res<DemoConfig>,
    mut menu_state: ResMut<MenuState>,
) {
    // Create simulation state
    commands.init_resource::<SimulationState>();
    
    // Set flags to hide menu and setup scene next frame
    menu_state.show_menu = false;
    menu_state.start_simulation = false;
    menu_state.setup_scene_next_frame = true;
}

// Setup the 3D scene
fn setup_scene(
    mut commands: Commands,
    mut menu_state: ResMut<MenuState>,
) {
    menu_state.setup_scene_next_frame = false;
    
    // Call the actual setup
    setup(&mut commands);
}

fn setup(
    commands: &mut Commands,
) {
    // Remove 2D camera, add 3D camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, -100.0, 80.0).looking_at(Vec3::ZERO, Vec3::Z),
        MainCamera,
    ));
    
    // Add light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));
    
    // Spawn terrain and fuel elements
    // (Implementation continues...)
}

// Placeholder for remaining functions
fn spawn_terrain(/* ... */) {}
fn update_fps(/* ... */) {}
fn update_simulation(/* ... */) {}
fn update_fuel_visuals(/* ... */) {}
fn update_camera_controls(/* ... */) {}
fn update_ui(/* ... */) {}
fn update_tooltip(/* ... */) {}
fn handle_controls(/* ... */) {}
