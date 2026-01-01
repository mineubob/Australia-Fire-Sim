//! Field queries FFI (C-compatible API)
#![allow(clippy::pedantic)]
#![allow(clippy::cast_slice_from_raw_parts)]
#![allow(clippy::borrow_as_ptr)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

/// FFI query functions for field-based fire simulation.
///
/// Provides C-compatible queries for fire front vertices, temperature grids,
/// level set fields, and simulation statistics.
use std::slice;

use crate::error::{DefaultFireSimError, FireSimErrorCode};
use crate::field_simulation::FireSimFieldInstance;
use crate::helpers::track_error;

/// C-compatible fire front vertex.
///
/// Represents a single point on the fire perimeter with derived quantities.
#[repr(C)]
pub struct FireFrontVertex {
    /// X position (meters)
    pub x: f32,
    /// Y position (meters)
    pub y: f32,
    /// Z position (meters)
    pub z: f32,
    /// Normal X component (unit vector pointing outward)
    pub normal_x: f32,
    /// Normal Y component (unit vector pointing outward)
    pub normal_y: f32,
    /// Spread velocity (m/s)
    pub velocity: f32,
    /// Byram fireline intensity (kW/m)
    pub intensity: f32,
    /// Local curvature (1/m, positive = convex, negative = concave)
    pub curvature: f32,
}

/// C-compatible fire front data.
///
/// Contains an array of vertices representing the fire perimeter.
/// May contain multiple disconnected fronts.
#[repr(C)]
pub struct FireFrontData {
    /// Array of vertices
    pub vertices: *mut FireFrontVertex,
    /// Number of vertices
    pub count: u32,
}

/// C-compatible simulation statistics.
#[repr(C)]
pub struct FieldSimStats {
    /// Burned area (square meters)
    pub burned_area_m2: f64,
    /// Fuel consumed (kilograms)
    pub fuel_consumed_kg: f64,
    /// Simulation time (seconds)
    pub simulation_time_s: f64,
    /// Number of active embers
    pub ember_count: u32,
    /// Whether GPU-accelerated
    pub is_gpu: bool,
}

/// Gets the current fire front as an array of vertices.
///
/// The fire front is extracted using marching squares on the level set field.
/// Vertices are ordered to form a polyline (or multiple disconnected polylines).
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `out_data` - Output pointer for FireFrontData (must not be null)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
/// If successful, *out_data contains pointer to FireFrontData.
/// Caller must call fire_sim_free_fire_front() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer from fire_sim_field_new()
/// - `out_data` must be a valid output pointer
#[no_mangle]
pub extern "C" fn fire_sim_field_get_fire_front(
    instance: *const FireSimFieldInstance,
    out_data: *mut *mut FireFrontData,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_data.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_data"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let fire_front = instance_ref.simulation.fire_front();

    // Convert to C-compatible format
    let vertices: Vec<FireFrontVertex> = fire_front
        .vertices
        .iter()
        .zip(fire_front.normals.iter())
        .zip(fire_front.velocities.iter())
        .zip(fire_front.intensities.iter())
        .zip(fire_front.curvatures.iter())
        .map(
            |((((vertex, normal), velocity), intensity), curvature)| FireFrontVertex {
                x: vertex.x,
                y: vertex.y,
                z: vertex.z,
                normal_x: normal.x,
                normal_y: normal.y,
                velocity: velocity.magnitude(),
                intensity: *intensity,
                curvature: *curvature,
            },
        )
        .collect();

    let count = vertices.len() as u32;
    let vertices_ptr = Box::into_raw(vertices.into_boxed_slice()) as *mut FireFrontVertex;

    let data = Box::into_raw(Box::new(FireFrontData {
        vertices: vertices_ptr,
        count,
    }));

    unsafe {
        *out_data = data;
    }

    FireSimErrorCode::Ok
}

/// Frees fire front data allocated by fire_sim_field_get_fire_front().
///
/// # Arguments
/// * `data` - The fire front data to free (can be null, no-op)
///
/// # Safety
/// `data` must have been created by fire_sim_field_get_fire_front()
#[no_mangle]
pub extern "C" fn fire_sim_free_fire_front(data: *mut FireFrontData) {
    if !data.is_null() {
        unsafe {
            let data_box = Box::from_raw(data);
            if !data_box.vertices.is_null() {
                let _ = Box::from_raw(slice::from_raw_parts_mut(
                    data_box.vertices,
                    data_box.count as usize,
                ));
            }
        }
    }
}

/// Gets the temperature grid (2D array of floats).
///
/// Returns the current temperature field in Celsius.
/// Grid is row-major: temperature[y * width + x].
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `out_grid` - Output: pointer to float array (must not be null)
/// * `out_width` - Output: grid width (must not be null)
/// * `out_height` - Output: grid height (must not be null)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
/// If successful, *out_grid contains pointer to float array of size (width * height).
/// Caller must call fire_sim_free_grid() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `out_grid`, `out_width`, and `out_height` must be valid output pointers
#[no_mangle]
pub extern "C" fn fire_sim_field_get_temperature_grid(
    instance: *const FireSimFieldInstance,
    out_grid: *mut *mut f32,
    out_width: *mut u32,
    out_height: *mut u32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_grid.is_null() || out_width.is_null() || out_height.is_null() {
        track_error(&DefaultFireSimError::null_pointer("output parameter"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let temperature_data = instance_ref.simulation.read_temperature();
    let (w, h, _cell_size) = instance_ref.simulation.grid_dimensions();

    // Copy data to heap-allocated array
    let grid_vec: Vec<f32> = temperature_data.iter().copied().collect();
    let grid_ptr = Box::into_raw(grid_vec.into_boxed_slice()) as *mut f32;

    unsafe {
        *out_grid = grid_ptr;
        *out_width = w;
        *out_height = h;
    }

    FireSimErrorCode::Ok
}

/// Gets the level set φ field (2D array of floats).
///
/// Returns the signed distance field where:
/// - φ < 0: burned region
/// - φ = 0: fire front
/// - φ > 0: unburned fuel
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `out_grid` - Output: pointer to float array (must not be null)
/// * `out_width` - Output: grid width (must not be null)
/// * `out_height` - Output: grid height (must not be null)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
/// If successful, *out_grid contains pointer to float array of size (width * height).
/// Caller must call fire_sim_free_grid() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `out_grid`, `out_width`, and `out_height` must be valid output pointers
#[no_mangle]
pub extern "C" fn fire_sim_field_read_level_set(
    instance: *const FireSimFieldInstance,
    out_grid: *mut *mut f32,
    out_width: *mut u32,
    out_height: *mut u32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_grid.is_null() || out_width.is_null() || out_height.is_null() {
        track_error(&DefaultFireSimError::null_pointer("output parameter"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let level_set_data = instance_ref.simulation.read_level_set();
    let (w, h, _cell_size) = instance_ref.simulation.grid_dimensions();

    // Copy data to heap-allocated array
    let grid_vec: Vec<f32> = level_set_data.iter().copied().collect();
    let grid_ptr = Box::into_raw(grid_vec.into_boxed_slice()) as *mut f32;

    unsafe {
        *out_grid = grid_ptr;
        *out_width = w;
        *out_height = h;
    }

    FireSimErrorCode::Ok
}

/// Frees grid data allocated by fire_sim_field_get_temperature_grid() or
/// fire_sim_field_read_level_set().
///
/// # Arguments
/// * `grid` - The grid data to free (can be null, no-op)
/// * `size` - The total number of elements (width * height)
///
/// # Safety
/// `grid` must have been created by fire_sim_field_get_temperature_grid() or similar
#[no_mangle]
pub extern "C" fn fire_sim_free_grid(grid: *mut f32, size: u32) {
    if !grid.is_null() && size > 0 {
        unsafe {
            let _ = Box::from_raw(slice::from_raw_parts_mut(grid, size as usize));
        }
    }
}

/// Gets simulation statistics.
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `out_stats` - Output pointer for FieldSimStats (must not be null)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
/// If successful, *out_stats contains pointer to FieldSimStats.
/// Caller must call fire_sim_free_stats() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer from fire_sim_field_new()
/// - `out_stats` must be a valid output pointer
#[no_mangle]
pub extern "C" fn fire_sim_field_get_stats(
    instance: *const FireSimFieldInstance,
    out_stats: *mut *mut FieldSimStats,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_stats.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_stats"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };

    let stats = FieldSimStats {
        burned_area_m2: instance_ref.simulation.burned_area() as f64,
        fuel_consumed_kg: instance_ref.simulation.fuel_consumed() as f64,
        simulation_time_s: instance_ref.simulation.simulation_time() as f64,
        ember_count: instance_ref.simulation.ember_count(),
        is_gpu: instance_ref.simulation.is_gpu_accelerated(),
    };

    let stats_ptr = Box::into_raw(Box::new(stats));

    unsafe {
        *out_stats = stats_ptr;
    }

    FireSimErrorCode::Ok
}

/// Frees statistics data allocated by fire_sim_field_get_stats().
///
/// # Arguments
/// * `stats` - The stats to free (can be null, no-op)
///
/// # Safety
/// `stats` must have been created by fire_sim_field_get_stats()
#[no_mangle]
pub extern "C" fn fire_sim_free_stats(stats: *mut FieldSimStats) {
    if !stats.is_null() {
        unsafe {
            let _ = Box::from_raw(stats);
        }
    }
}

// ============================================================================
// POINT QUERY API (Option B: Game engine polls fire state)
// ============================================================================

/// Query temperature at a specific world position.
///
/// Returns the temperature in Celsius at the given position.
/// Game objects can poll this to determine when to ignite.
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `x` - X position in meters
/// * `y` - Y position in meters
/// * `out_temp` - Output pointer for temperature in Celsius
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
///
/// # Safety
/// - `instance` must be a valid pointer from `fire_sim_field_new()`
/// - `out_temp` must be a valid output pointer
#[no_mangle]
pub extern "C" fn fire_sim_field_temperature_at(
    instance: *const FireSimFieldInstance,
    x: f32,
    y: f32,
    out_temp: *mut f32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_temp.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_temp"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let temp = instance_ref.simulation.temperature_at(x, y);

    unsafe {
        *out_temp = temp;
    }

    FireSimErrorCode::Ok
}

/// Query level set value at a specific world position.
///
/// Returns the signed distance from the fire front:
/// - φ < 0: Inside burned area
/// - φ = 0: At fire front
/// - φ > 0: Unburned fuel
///
/// # Arguments
/// * `instance` - The simulation instance
/// * `x` - X position in meters
/// * `y` - Y position in meters
/// * `out_phi` - Output pointer for level set value
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `out_phi` must be a valid output pointer
#[no_mangle]
pub extern "C" fn fire_sim_field_level_set_at(
    instance: *const FireSimFieldInstance,
    x: f32,
    y: f32,
    out_phi: *mut f32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_phi.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_phi"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let phi = instance_ref.simulation.level_set_at(x, y);

    unsafe {
        *out_phi = phi;
    }

    FireSimErrorCode::Ok
}

/// Check if a position is within the burned area.
///
/// # Arguments
/// * `instance` - The simulation instance
/// * `x` - X position in meters
/// * `y` - Y position in meters
/// * `out_burned` - Output: 1 if burned/burning, 0 otherwise
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `out_burned` must be a valid output pointer
#[no_mangle]
pub extern "C" fn fire_sim_field_is_burned(
    instance: *const FireSimFieldInstance,
    x: f32,
    y: f32,
    out_burned: *mut i32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if out_burned.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_burned"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &*instance };
    let burned = instance_ref.simulation.is_burned(x, y);

    unsafe {
        *out_burned = i32::from(burned);
    }

    FireSimErrorCode::Ok
}

/// Batch query temperatures at multiple positions.
///
/// This is more efficient than calling `fire_sim_field_temperature_at()` repeatedly.
///
/// # Arguments
/// * `instance` - The simulation instance
/// * `positions` - Pointer to array of floats [x0, y0, x1, y1, ...]
/// * `count` - Number of positions (array length / 2)
/// * `out_temps` - Output array of temperatures (must have `count` elements)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `positions` must point to `count * 2` floats
/// - `out_temps` must point to `count` floats
#[no_mangle]
pub extern "C" fn fire_sim_field_query_temperatures(
    instance: *const FireSimFieldInstance,
    positions: *const f32,
    count: u32,
    out_temps: *mut f32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if positions.is_null() {
        track_error(&DefaultFireSimError::null_pointer("positions"));
        return FireSimErrorCode::NullPointer;
    }
    if out_temps.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_temps"));
        return FireSimErrorCode::NullPointer;
    }
    if count == 0 {
        return FireSimErrorCode::Ok;
    }

    let instance_ref = unsafe { &*instance };
    let positions_slice = unsafe { slice::from_raw_parts(positions, (count * 2) as usize) };

    // Convert to tuples and query
    let pos_tuples: Vec<(f32, f32)> = positions_slice
        .chunks(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect();

    let temps = instance_ref.simulation.temperatures_at(&pos_tuples);

    // Copy to output
    unsafe {
        let out_slice = slice::from_raw_parts_mut(out_temps, count as usize);
        out_slice.copy_from_slice(&temps);
    }

    FireSimErrorCode::Ok
}

/// Batch query burn states at multiple positions.
///
/// # Arguments
/// * `instance` - The simulation instance
/// * `positions` - Pointer to array of floats [x0, y0, x1, y1, ...]
/// * `count` - Number of positions (array length / 2)
/// * `out_burned` - Output array of burn states (1 = burned, 0 = not burned)
///
/// # Returns
/// FireSimErrorCode::Ok on success, error code otherwise.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `positions` must point to `count * 2` floats
/// - `out_burned` must point to `count` i32 values
#[no_mangle]
pub extern "C" fn fire_sim_field_query_burn_states(
    instance: *const FireSimFieldInstance,
    positions: *const f32,
    count: u32,
    out_burned: *mut i32,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }
    if positions.is_null() {
        track_error(&DefaultFireSimError::null_pointer("positions"));
        return FireSimErrorCode::NullPointer;
    }
    if out_burned.is_null() {
        track_error(&DefaultFireSimError::null_pointer("out_burned"));
        return FireSimErrorCode::NullPointer;
    }
    if count == 0 {
        return FireSimErrorCode::Ok;
    }

    let instance_ref = unsafe { &*instance };
    let positions_slice = unsafe { slice::from_raw_parts(positions, (count * 2) as usize) };

    // Convert to tuples and query
    let pos_tuples: Vec<(f32, f32)> = positions_slice
        .chunks(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect();

    let burn_states = instance_ref.simulation.burn_states_at(&pos_tuples);

    // Copy to output
    unsafe {
        let out_slice = slice::from_raw_parts_mut(out_burned, count as usize);
        for (i, burned) in burn_states.iter().enumerate() {
            out_slice[i] = i32::from(*burned);
        }
    }

    FireSimErrorCode::Ok
}
