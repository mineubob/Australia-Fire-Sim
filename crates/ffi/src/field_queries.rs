/// FFI query functions for field-based fire simulation.
///
/// Provides C-compatible queries for fire front vertices, temperature grids,
/// level set fields, and simulation statistics.
use std::ptr;
use std::slice;

use crate::error::DefaultFireSimError;
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
///
/// # Returns
/// Pointer to FireFrontData, or null on error.
/// Caller must call fire_sim_free_fire_front() to free memory.
///
/// # Safety
/// `instance` must be a valid pointer from fire_sim_field_new()
#[no_mangle]
pub extern "C" fn fire_sim_field_get_fire_front(
    instance: *const FireSimFieldInstance,
) -> *mut FireFrontData {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return ptr::null_mut();
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

    Box::into_raw(Box::new(FireFrontData {
        vertices: vertices_ptr,
        count,
    }))
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
/// * `width` - Output: grid width (must not be null)
/// * `height` - Output: grid height (must not be null)
///
/// # Returns
/// Pointer to float array of size (width * height), or null on error.
/// Caller must call fire_sim_free_grid() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `width` and `height` must be valid output pointers
#[no_mangle]
pub extern "C" fn fire_sim_field_get_temperature_grid(
    instance: *const FireSimFieldInstance,
    width: *mut u32,
    height: *mut u32,
) -> *mut f32 {
    if instance.is_null() || width.is_null() || height.is_null() {
        track_error(&DefaultFireSimError::null_pointer("parameter"));
        return ptr::null_mut();
    }

    let instance_ref = unsafe { &*instance };
    let temperature_data = instance_ref.simulation.read_temperature();
    let (w, h, _cell_size) = instance_ref.simulation.grid_dimensions();

    unsafe {
        *width = w;
        *height = h;
    }

    // Copy data to heap-allocated array
    let grid_vec: Vec<f32> = temperature_data.iter().copied().collect();
    Box::into_raw(grid_vec.into_boxed_slice()) as *mut f32
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
/// * `width` - Output: grid width (must not be null)
/// * `height` - Output: grid height (must not be null)
///
/// # Returns
/// Pointer to float array of size (width * height), or null on error.
/// Caller must call fire_sim_free_grid() to free memory.
///
/// # Safety
/// - `instance` must be a valid pointer
/// - `width` and `height` must be valid output pointers
#[no_mangle]
pub extern "C" fn fire_sim_field_read_level_set(
    instance: *const FireSimFieldInstance,
    width: *mut u32,
    height: *mut u32,
) -> *mut f32 {
    if instance.is_null() || width.is_null() || height.is_null() {
        track_error(&DefaultFireSimError::null_pointer("parameter"));
        return ptr::null_mut();
    }

    let instance_ref = unsafe { &*instance };
    let level_set_data = instance_ref.simulation.read_level_set();
    let (w, h, _cell_size) = instance_ref.simulation.grid_dimensions();

    unsafe {
        *width = w;
        *height = h;
    }

    // Copy data to heap-allocated array
    let grid_vec: Vec<f32> = level_set_data.iter().copied().collect();
    Box::into_raw(grid_vec.into_boxed_slice()) as *mut f32
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
///
/// # Returns
/// Pointer to FieldSimStats, or null on error.
/// Caller must call fire_sim_free_stats() to free memory.
///
/// # Safety
/// `instance` must be a valid pointer from fire_sim_field_new()
#[no_mangle]
pub extern "C" fn fire_sim_field_get_stats(
    instance: *const FireSimFieldInstance,
) -> *mut FieldSimStats {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return ptr::null_mut();
    }

    let instance_ref = unsafe { &*instance };

    let stats = FieldSimStats {
        burned_area_m2: instance_ref.simulation.burned_area() as f64,
        fuel_consumed_kg: instance_ref.simulation.fuel_consumed() as f64,
        simulation_time_s: instance_ref.simulation.simulation_time() as f64,
        ember_count: instance_ref.simulation.ember_count(),
        is_gpu: instance_ref.simulation.is_gpu_accelerated(),
    };

    Box::into_raw(Box::new(stats))
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
