use fire_sim_core::{
    FireSimulation, Fuel, FuelPart, SuppressionAgent, TerrainData, Vec3,
};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

// Thread-safe simulation storage
static SIMULATIONS: LazyLock<Mutex<HashMap<usize, Arc<RwLock<FireSimulation>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static mut NEXT_SIM_ID: usize = 1;

// ============================================================================
// FFI ERROR CODES FOR C++ INTEGRATION
// ============================================================================

/// Success code
pub const FIRE_SIM_SUCCESS: i32 = 0;
/// Invalid simulation ID
pub const FIRE_SIM_INVALID_ID: i32 = -1;
/// Null pointer passed
pub const FIRE_SIM_NULL_POINTER: i32 = -2;
/// Invalid fuel type
pub const FIRE_SIM_INVALID_FUEL: i32 = -3;
/// Invalid terrain type
pub const FIRE_SIM_INVALID_TERRAIN: i32 = -4;
/// Lock error (internal)
pub const FIRE_SIM_LOCK_ERROR: i32 = -5;

// ============================================================================
// ULTRA-REALISTIC FIRE SIMULATION FFI
// ============================================================================

fn with_fire_sim_read<F, T>(id: &usize, func: F) -> Option<T>
where
    F: FnOnce(&FireSimulation) -> T,
{
    let simulation = {
        let simulations = SIMULATIONS.lock().unwrap();

        simulations.get(id)?.clone()
    };
    let simulation = simulation.read().unwrap();

    Some(func(&simulation))
}

fn with_fire_sim_write<F, T>(id: &usize, func: F) -> Option<T>
where
    F: FnOnce(&mut FireSimulation) -> T,
{
    let simulation = {
        let simulations = SIMULATIONS.lock().unwrap();

        simulations.get(id)?.clone()
    };
    let mut simulation = simulation.write().unwrap();

    Some(func(&mut simulation))
}

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
///
/// # Parameters
/// - `width`: World width in meters
/// - `height`: World height in meters
/// - `grid_cell_size`: Grid cell size in meters (typically 2-5m)
/// - `terrain_type`: 0=flat, 1=single_hill, 2=valley
/// - `out_sim_id`: Pointer to receive simulation ID
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success, with `out_sim_id` set
/// - `FIRE_SIM_INVALID_TERRAIN` (-4) if terrain_type is invalid
/// - `FIRE_SIM_NULL_POINTER` (-2) if out_sim_id is null
/// - `FIRE_SIM_LOCK_ERROR` (-5) if internal lock fails
///
/// # Safety
/// `out_sim_id` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_create(
    width: f32,
    height: f32,
    grid_cell_size: f32,
    terrain_type: u8,
    out_sim_id: *mut usize,
) -> i32 {
    if out_sim_id.is_null() {
        return FIRE_SIM_NULL_POINTER;
    }

    if terrain_type > 2 {
        return FIRE_SIM_INVALID_TERRAIN;
    }

    let terrain = match terrain_type {
        1 => TerrainData::single_hill(width, height, 5.0, 0.0, 100.0, width * 0.2),
        2 => TerrainData::valley_between_hills(width, height, 5.0, 0.0, 80.0),
        _ => TerrainData::flat(width, height, 5.0, 0.0),
    };

    let sim = FireSimulation::new(grid_cell_size, terrain);
    let sim_arc = Arc::new(RwLock::new(sim));

    let id = NEXT_SIM_ID;
    NEXT_SIM_ID += 1;

    match SIMULATIONS.lock() {
        Ok(mut sims) => {
            sims.insert(id, sim_arc);
            *out_sim_id = id;
            FIRE_SIM_SUCCESS
        }
        Err(_) => FIRE_SIM_LOCK_ERROR,
    }
}

/// Destroy an ultra-realistic simulation
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
/// - `FIRE_SIM_LOCK_ERROR` (-5) if internal lock fails
#[no_mangle]
pub extern "C" fn fire_sim_destroy(sim_id: usize) -> i32 {
    match SIMULATIONS.lock() {
        Ok(mut sims) => {
            if sims.remove(&sim_id).is_some() {
                FIRE_SIM_SUCCESS
            } else {
                FIRE_SIM_INVALID_ID
            }
        }
        Err(_) => FIRE_SIM_LOCK_ERROR,
    }
}

/// Update the ultra-realistic simulation
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
#[no_mangle]
pub extern "C" fn fire_sim_update(sim_id: usize, dt: f32) -> i32 {
    match with_fire_sim_write(&sim_id, |sim| sim.update(dt)) {
        Some(_) => FIRE_SIM_SUCCESS,
        None => FIRE_SIM_INVALID_ID,
    }
}

/// Add a fuel element to ultra simulation
///
/// # Parameters
/// - `fuel_type`: Fuel type ID (0-7)
/// - `part_type`: Fuel part type (0-7)
/// - `parent_id`: Parent element ID, or -1 for none
/// - `out_element_id`: Pointer to receive new element ID
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success, with `out_element_id` set
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
/// - `FIRE_SIM_INVALID_FUEL` (-3) if fuel_type is invalid
/// - `FIRE_SIM_NULL_POINTER` (-2) if out_element_id is null
///
/// # Safety
/// `out_element_id` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_add_fuel(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: i32,
    out_element_id: *mut u32,
) -> i32 {
    if out_element_id.is_null() {
        return FIRE_SIM_NULL_POINTER;
    }

    if fuel_type > 7 {
        return FIRE_SIM_INVALID_FUEL;
    }

    match with_fire_sim_write(&sim_id, |sim| {
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

        sim.add_fuel_element(position, fuel, mass, part, parent)
    }) {
        Some(id) => {
            *out_element_id = id;
            FIRE_SIM_SUCCESS
        }
        None => FIRE_SIM_INVALID_ID,
    }
}

/// Ignite a fuel element in ultra simulation
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
#[no_mangle]
pub extern "C" fn fire_sim_ignite(sim_id: usize, element_id: u32, initial_temp: f32) -> i32 {
    match with_fire_sim_write(&sim_id, |sim| sim.ignite_element(element_id, initial_temp)) {
        Some(_) => FIRE_SIM_SUCCESS,
        None => FIRE_SIM_INVALID_ID,
    }
}

/// Query elevation at world position
///
/// # Parameters
/// - `out_elevation`: Pointer to receive elevation value
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success, with `out_elevation` set
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
/// - `FIRE_SIM_NULL_POINTER` (-2) if out_elevation is null
///
/// # Safety
/// `out_elevation` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_elevation(
    sim_id: usize,
    x: f32,
    y: f32,
    out_elevation: *mut f32,
) -> i32 {
    if out_elevation.is_null() {
        return FIRE_SIM_NULL_POINTER;
    }

    match with_fire_sim_read(&sim_id, |sim| sim.grid.terrain.elevation_at(x, y)) {
        Some(elev) => {
            *out_elevation = elev;
            FIRE_SIM_SUCCESS
        }
        None => FIRE_SIM_INVALID_ID,
    }
}

/// Get grid cell state at world position
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success, with `out_cell` populated
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist or cell not found
/// - `FIRE_SIM_NULL_POINTER` (-2) if out_cell is null
///
/// # Safety
/// `out_cell` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_cell(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    out_cell: *mut GridCellVisual,
) -> i32 {
    if out_cell.is_null() {
        return FIRE_SIM_NULL_POINTER;
    }

    match with_fire_sim_read(&sim_id, |sim| {
        let pos = Vec3::new(x, y, z);
        if let Some(cell) = sim.get_cell_at_position(pos) {
            (*out_cell).temperature = cell.temperature();
            let wind = cell.wind();
            (*out_cell).wind_x = wind.x;
            (*out_cell).wind_y = wind.y;
            (*out_cell).wind_z = wind.z;
            (*out_cell).humidity = cell.humidity();
            (*out_cell).oxygen = cell.oxygen();
            (*out_cell).smoke_particles = cell.smoke_particles();
            (*out_cell).suppression_agent = cell.suppression_agent();
            return true;
        }
        false
    }) {
        Some(true) => FIRE_SIM_SUCCESS,
        _ => FIRE_SIM_INVALID_ID,
    }
}

/// Apply suppression directly at coordinates without physics simulation
///
/// This function immediately applies suppression agent at the specified coordinates,
/// bypassing the physics-based droplet simulation. Useful for direct application
/// such as ground crews or instant effects.
///
/// # Parameters
/// - `sim_id`: Simulation ID
/// - `x`, `y`, `z`: World coordinates where suppression is applied
/// - `mass`: Mass of suppression agent in kg
/// - `agent_type`: Type of suppression agent (0=Water, 1=ShortTermRetardant, 2=LongTermRetardant, 3=Foam)
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
/// - `FIRE_SIM_INVALID_FUEL` (-3) if agent_type is invalid
#[no_mangle]
pub extern "C" fn fire_sim_apply_suppression_direct(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    mass: f32,
    agent_type: u8,
) -> i32 {
    let agent = match agent_type {
        0 => SuppressionAgent::Water,
        1 => SuppressionAgent::ShortTermRetardant,
        2 => SuppressionAgent::LongTermRetardant,
        3 => SuppressionAgent::Foam,
        _ => return FIRE_SIM_INVALID_FUEL,
    };

    match with_fire_sim_write(&sim_id, |sim| {
        sim.apply_suppression_direct(Vec3::new(x, y, z), mass, agent);
    }) {
        Some(_) => FIRE_SIM_SUCCESS,
        None => FIRE_SIM_INVALID_ID,
    }
}

/// Apply water suppression directly at coordinates without physics simulation
///
/// Convenience function for water suppression. Same as `fire_sim_apply_suppression_direct`
/// with agent_type=0 (Water).
///
/// # Parameters
/// - `sim_id`: Simulation ID
/// - `x`, `y`, `z`: World coordinates where water is applied
/// - `mass`: Mass of water in kg
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
#[no_mangle]
pub extern "C" fn fire_sim_apply_water_direct(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    mass: f32,
) -> i32 {
    fire_sim_apply_suppression_direct(sim_id, x, y, z, mass, 0)
}

/// Get ultra simulation statistics
///
/// # Returns
/// - `FIRE_SIM_SUCCESS` (0) on success, with all pointers populated
/// - `FIRE_SIM_INVALID_ID` (-1) if sim_id doesn't exist
/// - `FIRE_SIM_NULL_POINTER` (-2) if any pointer is null
///
/// # Safety
/// All pointer parameters must be valid, non-null pointers
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_stats(
    sim_id: usize,
    out_burning: *mut u32,
    out_total: *mut u32,
    out_active_cells: *mut u32,
    out_total_cells: *mut u32,
    out_fuel_consumed: *mut f32,
) -> i32 {
    if out_burning.is_null()
        || out_total.is_null()
        || out_active_cells.is_null()
        || out_total_cells.is_null()
        || out_fuel_consumed.is_null()
    {
        return FIRE_SIM_NULL_POINTER;
    }

    match with_fire_sim_read(&sim_id, |sim| {
        let stats = sim.get_stats();
        *out_burning = stats.burning_elements as u32;
        *out_total = stats.total_elements as u32;
        *out_active_cells = stats.active_cells as u32;
        *out_total_cells = stats.total_cells as u32;
        *out_fuel_consumed = stats.total_fuel_consumed;
    }) {
        Some(_) => FIRE_SIM_SUCCESS,
        None => FIRE_SIM_INVALID_ID,
    }
}
