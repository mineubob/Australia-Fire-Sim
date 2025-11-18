use fire_sim_core::{
    FireSimulation, FireSimulationUltra, Fuel, FuelPart, SuppressionAgent, SuppressionDroplet,
    TerrainData, Vec3, WeatherSystem,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Thread-safe simulation storage
lazy_static::lazy_static! {
    static ref SIMULATIONS: Mutex<HashMap<usize, Arc<Mutex<FireSimulation>>>> = Mutex::new(HashMap::new());
    static ref ULTRA_SIMULATIONS: Mutex<HashMap<usize, Arc<Mutex<FireSimulationUltra>>>> = Mutex::new(HashMap::new());
}

static mut NEXT_SIM_ID: usize = 1;

/// C-compatible fire element visual data
#[repr(C)]
pub struct FireElementVisual {
    pub id: u32,
    pub position: [f32; 3],
    pub temperature: f32,
    pub flame_height: f32,
    pub intensity: f32,
    pub fuel_type_id: u8,
    pub part_type: u8,
}

/// C-compatible ember visual data
#[repr(C)]
pub struct EmberVisual {
    pub id: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub temperature: f32,
    pub size: f32,
}

/// Create a new fire simulation (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_create(width: f32, height: f32, depth: f32) -> usize {
    let sim = FireSimulation::new(width, height, depth);
    let sim_arc = Arc::new(Mutex::new(sim));

    unsafe {
        let id = NEXT_SIM_ID;
        NEXT_SIM_ID += 1;

        if let Ok(mut sims) = SIMULATIONS.lock() {
            sims.insert(id, sim_arc);
        }

        id
    }
}

/// Destroy a fire simulation (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_destroy(sim_id: usize) {
    if let Ok(mut sims) = SIMULATIONS.lock() {
        sims.remove(&sim_id);
    }
}

/// Update the simulation (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_update(sim_id: usize, dt: f32) {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                sim.update(dt);
            }
        }
    }
}

/// Add a fuel element to the simulation (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_add_fuel_element(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: i32,
) -> u32 {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                let position = Vec3::new(x, y, z);

                let fuel = Fuel::from_id(fuel_type).unwrap_or_else(Fuel::dry_grass);

                let part = match part_type {
                    0 => FuelPart::Root,
                    1 => FuelPart::TrunkLower,
                    2 => FuelPart::TrunkMiddle,
                    3 => FuelPart::TrunkUpper,
                    4 => FuelPart::Crown,
                    5 => FuelPart::GroundLitter,
                    6 => FuelPart::GroundVegetation,
                    _ => FuelPart::Surface,
                };

                let parent = if parent_id >= 0 {
                    Some(parent_id as u32)
                } else {
                    None
                };

                return sim.add_fuel_element(position, fuel, mass, part, parent);
            }
        }
    }
    0
}

/// Ignite a fuel element (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_ignite_element(sim_id: usize, element_id: u32, initial_temp: f32) {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                sim.ignite_element(element_id, initial_temp);
            }
        }
    }
}

/// Set weather conditions (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_set_weather(
    sim_id: usize,
    temp: f32,
    humidity: f32,
    wind_speed: f32,
    wind_direction: f32,
    drought: f32,
) {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                let weather =
                    WeatherSystem::new(temp, humidity, wind_speed, wind_direction, drought);
                sim.set_weather(weather);
            }
        }
    }
}

/// Update weather parameters without replacing (thread-safe)
#[no_mangle]
pub extern "C" fn fire_sim_update_weather(
    sim_id: usize,
    temp: f32,
    humidity: f32,
    wind_speed: f32,
    wind_direction: f32,
) {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                sim.weather.set_temperature(temp);
                sim.weather.set_humidity(humidity);
                sim.weather.set_wind_speed(wind_speed);
                sim.weather.set_wind_direction(wind_direction);
            }
        }
    }
}

/// Get burning elements for rendering (thread-safe)
/// # Safety
/// `out_count` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_burning_elements(
    sim_id: usize,
    out_count: *mut u32,
) -> *const FireElementVisual {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                let burning = sim.get_burning_elements();

                let visuals: Vec<FireElementVisual> = burning
                    .iter()
                    .map(|element| FireElementVisual {
                        id: element.id,
                        position: [element.position.x, element.position.y, element.position.z],
                        temperature: element.temperature,
                        flame_height: element.flame_height,
                        intensity: element.byram_fireline_intensity(),
                        fuel_type_id: element.fuel.id,
                        part_type: match element.part_type {
                            FuelPart::Root => 0,
                            FuelPart::TrunkLower => 1,
                            FuelPart::TrunkMiddle => 2,
                            FuelPart::TrunkUpper => 3,
                            FuelPart::Crown => 4,
                            FuelPart::GroundLitter => 5,
                            FuelPart::GroundVegetation => 6,
                            _ => 7,
                        },
                    })
                    .collect();

                unsafe {
                    *out_count = visuals.len() as u32;
                }

                let ptr = visuals.as_ptr();
                std::mem::forget(visuals); // Prevent deallocation
                return ptr;
            }
        }
    }

    *out_count = 0;
    std::ptr::null()
}

/// Get embers for particle effects (thread-safe)
/// # Safety
/// `out_count` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_embers(
    sim_id: usize,
    out_count: *mut u32,
) -> *const EmberVisual {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                let embers = sim.get_embers();

                let visuals: Vec<EmberVisual> = embers
                    .iter()
                    .map(|ember| EmberVisual {
                        id: ember.id,
                        position: [ember.position.x, ember.position.y, ember.position.z],
                        velocity: [ember.velocity.x, ember.velocity.y, ember.velocity.z],
                        temperature: ember.temperature,
                        size: (ember.mass * 1000.0).sqrt(), // Scale mass to visual size
                    })
                    .collect();

                unsafe {
                    *out_count = visuals.len() as u32;
                }

                let ptr = visuals.as_ptr();
                std::mem::forget(visuals);
                return ptr;
            }
        }
    }

    *out_count = 0;
    std::ptr::null()
}

/// Get simulation statistics (thread-safe)
/// # Safety
/// All pointer parameters must be valid, non-null pointers
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_stats(
    sim_id: usize,
    out_burning_count: *mut u32,
    out_ember_count: *mut u32,
    out_total_elements: *mut u32,
) {
    if let Ok(sims) = SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                unsafe {
                    *out_burning_count = sim.burning_count() as u32;
                    *out_ember_count = sim.ember_count() as u32;
                    *out_total_elements = sim.element_count() as u32;
                }
            }
        }
    }
}

/// Free memory allocated for element visuals
/// # Safety
/// `ptr` must be a valid pointer returned from fire_sim_get_burning_elements
#[no_mangle]
pub unsafe extern "C" fn fire_sim_free_elements(ptr: *mut FireElementVisual, count: u32) {
    if !ptr.is_null() {
        let _ = Vec::from_raw_parts(ptr, count as usize, count as usize);
    }
}

/// Free memory allocated for ember visuals
/// # Safety
/// `ptr` must be a valid pointer returned from fire_sim_get_embers
#[no_mangle]
pub unsafe extern "C" fn fire_sim_free_embers(ptr: *mut EmberVisual, count: u32) {
    if !ptr.is_null() {
        let _ = Vec::from_raw_parts(ptr, count as usize, count as usize);
    }
}

// ============================================================================
// ULTRA-REALISTIC FIRE SIMULATION FFI
// ============================================================================

/// C-compatible grid cell data
#[repr(C)]
pub struct GridCellVisual {
    pub temperature: f32,
    pub wind_x: f32,
    pub wind_y: f32,
    pub wind_z: f32,
    pub humidity: f32,
    pub oxygen: f32,
    pub smoke_particles: f32,
    pub suppression_agent: f32,
}

/// Create a new ultra-realistic fire simulation
#[no_mangle]
pub extern "C" fn fire_sim_ultra_create(
    width: f32,
    height: f32,
    _depth: f32, // Depth now computed from terrain
    grid_cell_size: f32,
    terrain_type: u8, // 0=flat, 1=single_hill, 2=valley
) -> usize {
    let terrain = match terrain_type {
        1 => TerrainData::single_hill(width, height, 5.0, 0.0, 100.0, width * 0.2),
        2 => TerrainData::valley_between_hills(width, height, 5.0, 0.0, 80.0),
        _ => TerrainData::flat(width, height, 5.0, 0.0),
    };

    let sim = FireSimulationUltra::new(grid_cell_size, terrain);
    let sim_arc = Arc::new(Mutex::new(sim));

    unsafe {
        let id = NEXT_SIM_ID;
        NEXT_SIM_ID += 1;

        if let Ok(mut sims) = ULTRA_SIMULATIONS.lock() {
            sims.insert(id, sim_arc);
        }

        id
    }
}

/// Destroy an ultra-realistic simulation
#[no_mangle]
pub extern "C" fn fire_sim_ultra_destroy(sim_id: usize) {
    if let Ok(mut sims) = ULTRA_SIMULATIONS.lock() {
        sims.remove(&sim_id);
    }
}

/// Update the ultra-realistic simulation
#[no_mangle]
pub extern "C" fn fire_sim_ultra_update(sim_id: usize, dt: f32) {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                sim.update(dt);
            }
        }
    }
}

/// Add a fuel element to ultra simulation
#[no_mangle]
pub extern "C" fn fire_sim_ultra_add_fuel(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: i32,
) -> u32 {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                let position = Vec3::new(x, y, z);
                let fuel = Fuel::from_id(fuel_type).unwrap_or_else(Fuel::dry_grass);

                let part = match part_type {
                    0 => FuelPart::Root,
                    1 => FuelPart::TrunkLower,
                    2 => FuelPart::TrunkMiddle,
                    3 => FuelPart::TrunkUpper,
                    4 => FuelPart::Crown,
                    5 => FuelPart::GroundLitter,
                    6 => FuelPart::GroundVegetation,
                    _ => FuelPart::Surface,
                };

                let parent = if parent_id >= 0 {
                    Some(parent_id as u32)
                } else {
                    None
                };

                return sim.add_fuel_element(position, fuel, mass, part, parent);
            }
        }
    }
    0
}

/// Ignite a fuel element in ultra simulation
#[no_mangle]
pub extern "C" fn fire_sim_ultra_ignite(sim_id: usize, element_id: u32, initial_temp: f32) {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                sim.ignite_element(element_id, initial_temp);
            }
        }
    }
}

/// Query elevation at world position
#[no_mangle]
pub extern "C" fn fire_sim_ultra_get_elevation(sim_id: usize, x: f32, y: f32) -> f32 {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                return sim.grid.terrain.elevation_at(x, y);
            }
        }
    }
    0.0
}

/// Get grid cell state at world position
/// # Safety
/// `out_cell` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_ultra_get_cell(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    out_cell: *mut GridCellVisual,
) -> bool {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                let pos = Vec3::new(x, y, z);
                if let Some(cell) = sim.get_cell_at_position(pos) {
                    (*out_cell).temperature = cell.temperature;
                    (*out_cell).wind_x = cell.wind.x;
                    (*out_cell).wind_y = cell.wind.y;
                    (*out_cell).wind_z = cell.wind.z;
                    (*out_cell).humidity = cell.humidity;
                    (*out_cell).oxygen = cell.oxygen;
                    (*out_cell).smoke_particles = cell.smoke_particles;
                    (*out_cell).suppression_agent = cell.suppression_agent;
                    return true;
                }
            }
        }
    }
    false
}

/// Add water suppression droplet
#[no_mangle]
pub extern "C" fn fire_sim_ultra_add_water_drop(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    mass: f32,
) {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(mut sim) = sim_arc.lock() {
                let droplet = SuppressionDroplet::new(
                    Vec3::new(x, y, z),
                    Vec3::new(vx, vy, vz),
                    mass,
                    SuppressionAgent::Water,
                );
                sim.add_suppression_droplet(droplet);
            }
        }
    }
}

/// Get ultra simulation statistics
/// # Safety
/// All pointer parameters must be valid, non-null pointers
#[no_mangle]
pub unsafe extern "C" fn fire_sim_ultra_get_stats(
    sim_id: usize,
    out_burning: *mut u32,
    out_total: *mut u32,
    out_active_cells: *mut u32,
    out_total_cells: *mut u32,
    out_fuel_consumed: *mut f32,
) {
    if let Ok(sims) = ULTRA_SIMULATIONS.lock() {
        if let Some(sim_arc) = sims.get(&sim_id) {
            if let Ok(sim) = sim_arc.lock() {
                let stats = sim.get_stats();
                *out_burning = stats.burning_elements as u32;
                *out_total = stats.total_elements as u32;
                *out_active_cells = stats.active_cells as u32;
                *out_total_cells = stats.total_cells as u32;
                *out_fuel_consumed = stats.total_fuel_consumed;
            }
        }
    }
}
