use fire_sim_core::{FireSimulation, Fuel, FuelPart, Vec3, WeatherSystem};
use std::os::raw::c_char;
use std::ffi::CStr;

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

/// Create a new fire simulation
#[no_mangle]
pub extern "C" fn fire_sim_create(width: f32, height: f32, depth: f32) -> *mut FireSimulation {
    let sim = Box::new(FireSimulation::new(width, height, depth));
    Box::into_raw(sim)
}

/// Destroy a fire simulation
#[no_mangle]
pub extern "C" fn fire_sim_destroy(sim: *mut FireSimulation) {
    if !sim.is_null() {
        unsafe {
            let _ = Box::from_raw(sim);
        }
    }
}

/// Update the simulation
#[no_mangle]
pub extern "C" fn fire_sim_update(sim: *mut FireSimulation, dt: f32) {
    if let Some(sim) = unsafe { sim.as_mut() } {
        sim.update(dt);
    }
}

/// Add a fuel element to the simulation
#[no_mangle]
pub extern "C" fn fire_sim_add_fuel_element(
    sim: *mut FireSimulation,
    x: f32,
    y: f32,
    z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: i32,
) -> u32 {
    if let Some(sim) = unsafe { sim.as_mut() } {
        let position = Vec3::new(x, y, z);
        
        let fuel = Fuel::from_id(fuel_type).unwrap_or_else(|| Fuel::dry_grass());
        
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
    } else {
        0
    }
}

/// Ignite a fuel element
#[no_mangle]
pub extern "C" fn fire_sim_ignite_element(
    sim: *mut FireSimulation,
    element_id: u32,
    initial_temp: f32,
) {
    if let Some(sim) = unsafe { sim.as_mut() } {
        sim.ignite_element(element_id, initial_temp);
    }
}

/// Set weather conditions
#[no_mangle]
pub extern "C" fn fire_sim_set_weather(
    sim: *mut FireSimulation,
    temp: f32,
    humidity: f32,
    wind_speed: f32,
    wind_direction: f32,
    drought: f32,
) {
    if let Some(sim) = unsafe { sim.as_mut() } {
        let weather = WeatherSystem::new(temp, humidity, wind_speed, wind_direction, drought);
        sim.set_weather(weather);
    }
}

/// Get burning elements for rendering
#[no_mangle]
pub extern "C" fn fire_sim_get_burning_elements(
    sim: *mut FireSimulation,
    out_count: *mut u32,
) -> *const FireElementVisual {
    if let Some(sim) = unsafe { sim.as_mut() } {
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
        ptr
    } else {
        unsafe {
            *out_count = 0;
        }
        std::ptr::null()
    }
}

/// Get embers for particle effects
#[no_mangle]
pub extern "C" fn fire_sim_get_embers(
    sim: *mut FireSimulation,
    out_count: *mut u32,
) -> *const EmberVisual {
    if let Some(sim) = unsafe { sim.as_mut() } {
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
        ptr
    } else {
        unsafe {
            *out_count = 0;
        }
        std::ptr::null()
    }
}

/// Get simulation statistics
#[no_mangle]
pub extern "C" fn fire_sim_get_stats(
    sim: *mut FireSimulation,
    out_burning_count: *mut u32,
    out_ember_count: *mut u32,
    out_total_elements: *mut u32,
) {
    if let Some(sim) = unsafe { sim.as_mut() } {
        unsafe {
            *out_burning_count = sim.burning_count() as u32;
            *out_ember_count = sim.ember_count() as u32;
            *out_total_elements = sim.element_count() as u32;
        }
    }
}

/// Free memory allocated for element visuals
#[no_mangle]
pub extern "C" fn fire_sim_free_elements(ptr: *mut FireElementVisual, count: u32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, count as usize, count as usize);
        }
    }
}

/// Free memory allocated for ember visuals
#[no_mangle]
pub extern "C" fn fire_sim_free_embers(ptr: *mut EmberVisual, count: u32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, count as usize, count as usize);
        }
    }
}
