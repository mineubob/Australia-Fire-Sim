//! Field simulation FFI (C-compatible API)
#![allow(clippy::pedantic)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

/// FFI bindings for field-based fire simulation (FieldSimulation).
///
/// This module provides C-compatible API for the new GPU/CPU field solver system.
/// It coexists with the legacy element-based API for backward compatibility.
use fire_sim_core::core_types::element::Vec3;
use fire_sim_core::simulation::FieldSimulation;
use fire_sim_core::solver::QualityPreset;
use fire_sim_core::{TerrainData, WeatherSystem};
use std::ptr;

use crate::error::{DefaultFireSimError, FireSimErrorCode};
use crate::helpers::track_error;
use crate::terrain::Terrain;

/// Opaque handle to a field-based fire simulation instance.
///
/// This is the C-compatible handle for FieldSimulation. Memory is managed
/// by fire_sim_field_new/fire_sim_field_destroy.
pub struct FireSimFieldInstance {
    pub(crate) simulation: FieldSimulation,
}

/// Creates a new field-based fire simulation instance.
///
/// # Arguments
/// * `terrain` - Terrain configuration (must not be null)
/// * `quality` - Quality preset: 0=Low, 1=Medium, 2=High, 3=Ultra
/// * `temperature` - Ambient temperature (°C)
/// * `humidity` - Relative humidity (0.0-1.0)
/// * `wind_speed` - Wind speed (m/s)
/// * `wind_direction` - Wind direction (radians, 0 = east, π/2 = north)
/// * `wind_slope` - Wind slope angle (radians)
///
/// # Returns
/// Pointer to FireSimFieldInstance, or null on error.
/// Check fire_sim_get_last_error() for details.
///
/// # Safety
/// - `terrain` must be a valid pointer
/// - Caller owns returned pointer and must call fire_sim_field_destroy()
#[no_mangle]
pub extern "C" fn fire_sim_field_new(
    terrain: *const Terrain,
    quality: u8,
    temperature: f64,
    humidity: f64,
    wind_speed: f64,
    wind_direction: f64,
    wind_slope: f64,
) -> *mut FireSimFieldInstance {
    // Validate terrain pointer
    if terrain.is_null() {
        track_error(&DefaultFireSimError::null_pointer("terrain"));
        return ptr::null_mut();
    }

    // Convert quality preset
    let quality_preset = match quality {
        0 => QualityPreset::Low,
        1 => QualityPreset::Medium,
        2 => QualityPreset::High,
        3 => QualityPreset::Ultra,
        _ => {
            track_error(&DefaultFireSimError::invalid_parameter(format!(
                "Invalid quality preset: {}. Must be 0-3",
                quality
            )));
            return ptr::null_mut();
        }
    };

    // Create TerrainData from Terrain enum
    let terrain_ref = unsafe { &*terrain };
    let terrain_data = match terrain_ref {
        Terrain::Flat {
            width,
            height,
            resolution,
            base_elevation,
        } => TerrainData::flat(*width, *height, *resolution, *base_elevation),
        _ => {
            track_error(&DefaultFireSimError::invalid_parameter(
                "Only Flat terrain is currently supported for field simulation".to_string(),
            ));
            return ptr::null_mut();
        }
    };

    // Create WeatherSystem
    let weather = WeatherSystem::new(
        temperature as f32,
        humidity as f32,
        wind_speed as f32,
        wind_direction as f32,
        wind_slope as f32,
    );

    // Create FieldSimulation
    let simulation = FieldSimulation::new(&terrain_data, quality_preset, weather);

    // Wrap in FFI handle
    Box::into_raw(Box::new(FireSimFieldInstance { simulation }))
}

/// Destroys a field simulation instance and frees all memory.
///
/// # Arguments
/// * `instance` - The instance to destroy (can be null, no-op)
///
/// # Safety
/// - `instance` must have been created by fire_sim_field_new()
/// - After this call, `instance` pointer is invalid
#[no_mangle]
pub extern "C" fn fire_sim_field_destroy(instance: *mut FireSimFieldInstance) {
    if !instance.is_null() {
        unsafe {
            let _ = Box::from_raw(instance);
        }
    }
}

/// Updates the field simulation by one timestep.
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `dt` - Timestep in seconds (typically 0.1 to 1.0)
///
/// # Returns
/// FireSimErrorCode::Success on success, error code otherwise.
///
/// # Safety
/// `instance` must be a valid pointer from fire_sim_field_new()
#[no_mangle]
pub extern "C" fn fire_sim_field_update(
    instance: *mut FireSimFieldInstance,
    dt: f64,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &mut *instance };
    instance_ref.simulation.update(dt as f32);

    FireSimErrorCode::Ok
}

/// Ignites fire at a specified position with given radius.
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
/// * `x`, `y`, `z` - Position in meters
/// * `radius` - Ignition radius in meters
///
/// # Returns
/// FireSimErrorCode::Success on success, error code otherwise.
///
/// # Safety
/// `instance` must be a valid pointer from fire_sim_field_new()
#[no_mangle]
pub extern "C" fn fire_sim_field_ignite_at(
    instance: *mut FireSimFieldInstance,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
) -> FireSimErrorCode {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return FireSimErrorCode::NullPointer;
    }

    let instance_ref = unsafe { &mut *instance };
    let position = Vec3::new(x as f32, y as f32, z as f32);
    instance_ref.simulation.ignite_at(position, radius as f32);

    FireSimErrorCode::Ok
}

/// Returns whether the simulation is using GPU acceleration.
///
/// # Arguments
/// * `instance` - The simulation instance (must not be null)
///
/// # Returns
/// true if GPU-accelerated, false if CPU-only or on error.
///
/// # Safety
/// `instance` must be a valid pointer from fire_sim_field_new()
#[no_mangle]
pub extern "C" fn fire_sim_field_is_gpu_accelerated(instance: *const FireSimFieldInstance) -> bool {
    if instance.is_null() {
        track_error(&DefaultFireSimError::null_pointer("instance"));
        return false;
    }

    let instance_ref = unsafe { &*instance };
    instance_ref.simulation.is_gpu_accelerated()
}
