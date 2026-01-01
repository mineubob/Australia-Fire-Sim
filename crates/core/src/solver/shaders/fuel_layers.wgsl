// Vertical Fuel Layer Heat Transfer Shader
//
// Implements Stefan-Boltzmann radiative transfer and convective heat flux
// between burning lower layers and unburned upper layers.
//
// Physics:
// - Radiative transfer: Q_rad = ε × σ × (T_source⁴ - T_target⁴) × F_view
// - Convective transfer: Q_conv = h × (T_flame - T_target) × F_plume
// - Moisture evaporation BEFORE temperature rise (2.26 MJ/kg latent heat)
//
// Fuel layers:
// - Surface (0-0.5m): litter, grass, herbs
// - Shrub (0.5-3m): understory, bark (ladder fuels)
// - Canopy (3m+): tree crowns
//
// References:
// - "Fundamentals of Heat and Mass Transfer" (Incropera et al., 2002)
// - "An Introduction to Fire Dynamics" (Drysdale, 2011)
// - Albini (1986) "Wildland Fire Spread by Radiation"

// Physical constants
const STEFAN_BOLTZMANN: f32 = 5.67e-8;        // W/m²K⁴
const LATENT_HEAT_WATER: f32 = 2260000.0;     // J/kg
const IGNITION_TEMP: f32 = 573.15;            // K (~300°C)
const FLAME_TEMP: f32 = 1073.15;              // K (~800°C)

// Layer indices
const LAYER_SURFACE: u32 = 0u;
const LAYER_SHRUB: u32 = 1u;
const LAYER_CANOPY: u32 = 2u;

// Layer representative heights (m)
const HEIGHT_SURFACE: f32 = 0.25;
const HEIGHT_SHRUB: f32 = 1.75;
const HEIGHT_CANOPY: f32 = 10.0;

struct FuelLayerParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    // Heat transfer coefficients
    emissivity: f32,              // Effective emissivity (0.9 typical)
    convective_coeff_base: f32,   // Base convective coefficient (W/m²K)
    // Flame properties
    flame_height: f32,            // Current flame height (m)
    canopy_cover_fraction: f32,   // Affects radiative view factor
    // Fuel specific heat capacity
    fuel_specific_heat: f32,      // J/(kg·K), typically ~1500
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(0) var<uniform> params: FuelLayerParams;

// Per-layer arrays: each cell has 3 values (surface, shrub, canopy)
// Layout: [cell0_surface, cell0_shrub, cell0_canopy, cell1_surface, ...]
@group(0) @binding(1) var<storage, read_write> layer_fuel: array<f32>;      // kg/m²
@group(0) @binding(2) var<storage, read_write> layer_moisture: array<f32>;  // fraction 0-1
@group(0) @binding(3) var<storage, read_write> layer_temp: array<f32>;      // Kelvin
@group(0) @binding(4) var<storage, read_write> layer_burning: array<u32>;   // 0 or 1

// Surface fire intensity from crown_fire shader (for shrub ignition check)
@group(0) @binding(5) var<storage, read> fire_intensity: array<f32>;

// Helper to get layer data index
fn layer_idx(cell_idx: u32, layer: u32) -> u32 {
    return cell_idx * 3u + layer;
}

// Calculate view factor based on layer geometry and flame height
fn calculate_view_factor(source_height: f32, target_height: f32, flame_height: f32) -> f32 {
    let separation = abs(target_height - source_height);
    
    // Only upward heat transfer (source below target)
    if (source_height >= target_height) {
        return 0.0;
    }
    
    // Check if flame reaches target layer
    if (source_height + flame_height < target_height) {
        return 0.0;  // Flame doesn't reach
    }
    
    // View factor decreases with square of distance, modified by canopy cover
    let distance_factor = 1.0 / (1.0 + separation * separation);
    let flame_coverage = min((source_height + flame_height - target_height) / target_height, 1.0);
    
    return distance_factor * flame_coverage * (1.0 - 0.5 * params.canopy_cover_fraction);
}

// Calculate plume fraction reaching target layer
fn calculate_plume_fraction(source_height: f32, target_height: f32, flame_height: f32) -> f32 {
    let separation = target_height - source_height;
    
    if (separation <= 0.0) {
        return 0.0;  // No downward convection
    }
    
    // Plume entrains air and cools with height
    // Fraction decreases exponentially with height
    let plume_reach = flame_height * 1.5;  // Plume extends beyond visible flame
    
    if (separation > plume_reach) {
        return 0.0;
    }
    
    return exp(-separation / plume_reach);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    
    if (x >= params.width || y >= params.height) {
        return;
    }
    
    let cell_idx = y * params.width + x;
    let intensity = fire_intensity[cell_idx];
    
    // Process each layer pair (surface→shrub, shrub→canopy)
    // Only transfer heat upward from burning layers
    
    // Surface layer state
    let surf_idx = layer_idx(cell_idx, LAYER_SURFACE);
    let surf_fuel = layer_fuel[surf_idx];
    let surf_temp = layer_temp[surf_idx];
    let surf_burning = layer_burning[surf_idx];
    
    // Shrub layer state
    let shrub_idx = layer_idx(cell_idx, LAYER_SHRUB);
    var shrub_fuel = layer_fuel[shrub_idx];
    var shrub_moisture = layer_moisture[shrub_idx];
    var shrub_temp = layer_temp[shrub_idx];
    var shrub_burning = layer_burning[shrub_idx];
    
    // Canopy layer state
    let canopy_idx = layer_idx(cell_idx, LAYER_CANOPY);
    var canopy_fuel = layer_fuel[canopy_idx];
    var canopy_moisture = layer_moisture[canopy_idx];
    var canopy_temp = layer_temp[canopy_idx];
    var canopy_burning = layer_burning[canopy_idx];
    
    // === Surface → Shrub heat transfer ===
    if (surf_burning == 1u && shrub_fuel > 1e-6) {
        let view_factor = calculate_view_factor(HEIGHT_SURFACE, HEIGHT_SHRUB, params.flame_height);
        let plume_fraction = calculate_plume_fraction(HEIGHT_SURFACE, HEIGHT_SHRUB, params.flame_height);
        
        // Radiative heat flux: Q_rad = ε × σ × (T_source⁴ - T_target⁴) × F_view
        let t_source = max(surf_temp, FLAME_TEMP);  // Use flame temp if burning
        let t_target = shrub_temp;
        let t_source_4 = pow(t_source, 4.0);
        let t_target_4 = pow(t_target, 4.0);
        let q_rad = params.emissivity * STEFAN_BOLTZMANN * (t_source_4 - t_target_4) * view_factor;
        
        // Convective heat flux: Q_conv = h × (T_flame - T_target) × F_plume
        let q_conv = params.convective_coeff_base * (FLAME_TEMP - t_target) * plume_fraction;
        
        // Total heat flux (W/m²)
        let q_total = max(q_rad + q_conv, 0.0);
        
        // Convert to energy per timestep (J/m²)
        let energy = q_total * params.dt;
        
        // Apply heat: moisture evaporation FIRST, then temperature rise
        if (shrub_moisture > 1e-6 && energy > 0.0) {
            // Energy needed to evaporate all moisture
            let water_mass = shrub_fuel * shrub_moisture;
            let evap_energy = water_mass * LATENT_HEAT_WATER;
            
            if (energy >= evap_energy) {
                // All moisture evaporated, remaining energy heats fuel
                let remaining = energy - evap_energy;
                shrub_moisture = 0.0;
                shrub_temp = shrub_temp + remaining / (shrub_fuel * params.fuel_specific_heat);
            } else {
                // Partial evaporation
                let evap_fraction = energy / evap_energy;
                shrub_moisture = shrub_moisture * (1.0 - evap_fraction);
            }
        } else if (energy > 0.0) {
            // No moisture - direct temperature rise
            shrub_temp = shrub_temp + energy / (shrub_fuel * params.fuel_specific_heat);
        }
        
        // Check shrub ignition: intensity > 500 kW/m threshold
        if (shrub_burning == 0u && intensity >= 500.0 && shrub_temp >= IGNITION_TEMP) {
            shrub_burning = 1u;
        }
    }
    
    // === Shrub → Canopy heat transfer ===
    if (shrub_burning == 1u && canopy_fuel > 1e-6) {
        let view_factor = calculate_view_factor(HEIGHT_SHRUB, HEIGHT_CANOPY, params.flame_height);
        let plume_fraction = calculate_plume_fraction(HEIGHT_SHRUB, HEIGHT_CANOPY, params.flame_height);
        
        let t_source = max(shrub_temp, FLAME_TEMP);
        let t_target = canopy_temp;
        let t_source_4 = pow(t_source, 4.0);
        let t_target_4 = pow(t_target, 4.0);
        let q_rad = params.emissivity * STEFAN_BOLTZMANN * (t_source_4 - t_target_4) * view_factor;
        let q_conv = params.convective_coeff_base * (FLAME_TEMP - t_target) * plume_fraction;
        let q_total = max(q_rad + q_conv, 0.0);
        let energy = q_total * params.dt;
        
        if (canopy_moisture > 1e-6 && energy > 0.0) {
            let water_mass = canopy_fuel * canopy_moisture;
            let evap_energy = water_mass * LATENT_HEAT_WATER;
            
            if (energy >= evap_energy) {
                let remaining = energy - evap_energy;
                canopy_moisture = 0.0;
                canopy_temp = canopy_temp + remaining / (canopy_fuel * params.fuel_specific_heat);
            } else {
                let evap_fraction = energy / evap_energy;
                canopy_moisture = canopy_moisture * (1.0 - evap_fraction);
            }
        } else if (energy > 0.0) {
            canopy_temp = canopy_temp + energy / (canopy_fuel * params.fuel_specific_heat);
        }
        
        // Canopy ignition handled by Van Wagner criterion in crown_fire shader
    }
    
    // Write back updated layer states
    layer_moisture[shrub_idx] = shrub_moisture;
    layer_temp[shrub_idx] = shrub_temp;
    layer_burning[shrub_idx] = shrub_burning;
    
    layer_moisture[canopy_idx] = canopy_moisture;
    layer_temp[canopy_idx] = canopy_temp;
    layer_burning[canopy_idx] = canopy_burning;
}
