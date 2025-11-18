use fire_sim_core::{
    FireSimulation, Fuel, FuelPart, SuppressionAgent, SuppressionDroplet, TerrainData, Vec3,
};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

// Thread-safe simulation storage
static SIMULATIONS: LazyLock<Mutex<HashMap<usize, Arc<RwLock<FireSimulation>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static mut NEXT_SIM_ID: usize = 1;

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
#[no_mangle]
pub extern "C" fn fire_sim_create(
    width: f32,
    height: f32,
    grid_cell_size: f32,
    terrain_type: u8, // 0=flat, 1=single_hill, 2=valley
) -> usize {
    let terrain = match terrain_type {
        1 => TerrainData::single_hill(width, height, 5.0, 0.0, 100.0, width * 0.2),
        2 => TerrainData::valley_between_hills(width, height, 5.0, 0.0, 80.0),
        _ => TerrainData::flat(width, height, 5.0, 0.0),
    };

    let sim = FireSimulation::new(grid_cell_size, terrain);
    let sim_arc = Arc::new(RwLock::new(sim));

    unsafe {
        let id = NEXT_SIM_ID;
        NEXT_SIM_ID += 1;

        if let Ok(mut sims) = SIMULATIONS.lock() {
            sims.insert(id, sim_arc);
        }

        id
    }
}

/// Destroy an ultra-realistic simulation
#[no_mangle]
pub extern "C" fn fire_sim_destroy(sim_id: usize) {
    if let Ok(mut sims) = SIMULATIONS.lock() {
        sims.remove(&sim_id);
    }
}

/// Update the ultra-realistic simulation
#[no_mangle]
pub extern "C" fn fire_sim_update(sim_id: usize, dt: f32) {
    with_fire_sim_write(&sim_id, |sim| sim.update(dt));
}

/// Add a fuel element to ultra simulation
#[no_mangle]
pub extern "C" fn fire_sim_add_fuel(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: i32,
) -> u32 {
    with_fire_sim_write(&sim_id, |sim| {
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
    })
    .unwrap_or(0)
}

/// Ignite a fuel element in ultra simulation
#[no_mangle]
pub extern "C" fn fire_sim_ignite(sim_id: usize, element_id: u32, initial_temp: f32) {
    with_fire_sim_write(&sim_id, |sim| sim.ignite_element(element_id, initial_temp));
}

/// Query elevation at world position
#[no_mangle]
pub extern "C" fn fire_sim_get_elevation(sim_id: usize, x: f32, y: f32) -> f32 {
    with_fire_sim_read(&sim_id, |sim| sim.grid.terrain.elevation_at(x, y)).unwrap_or(0.0)
}

/// Get grid cell state at world position
/// # Safety
/// `out_cell` must be a valid, non-null pointer
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_cell(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    out_cell: *mut GridCellVisual,
) -> bool {
    with_fire_sim_read(&sim_id, |sim| {
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

        false
    })
    .unwrap_or(false)
}

/// Add water suppression droplet
#[no_mangle]
pub extern "C" fn fire_sim_add_water_drop(
    sim_id: usize,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    mass: f32,
) {
    with_fire_sim_write(&sim_id, |sim| {
        let droplet = SuppressionDroplet::new(
            Vec3::new(x, y, z),
            Vec3::new(vx, vy, vz),
            mass,
            SuppressionAgent::Water,
        );
        sim.add_suppression_droplet(droplet);
    });
}

/// Get ultra simulation statistics
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
) -> bool {
    with_fire_sim_read(&sim_id, |sim| {
        let stats = sim.get_stats();
        *out_burning = stats.burning_elements as u32;
        *out_total = stats.total_elements as u32;
        *out_active_cells = stats.active_cells as u32;
        *out_total_cells = stats.total_cells as u32;
        *out_fuel_consumed = stats.total_fuel_consumed;
    })
    .is_some()
}
