// Combustion compute shader for GPU
//
// Implements:
// - Moisture evaporation FIRST (2260 kJ/kg latent heat)
// - Fuel consumption (temperature, moisture, oxygen dependent)
// - Heat release calculation
// - Oxygen depletion (stoichiometric ratio)

struct CombustionParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    // Fuel-specific properties (from Rust Fuel type)
    ignition_temp_k: f32,       // Ignition temperature in Kelvin
    moisture_extinction: f32,   // Moisture of extinction (fraction)
    heat_content_kj: f32,       // Heat content in kJ/kg
    self_heating_fraction: f32, // Fraction of heat retained (0-1)
    burn_rate_coefficient: f32, // Base burn rate coefficient
    ambient_temp_k: f32,        // Ambient temperature in Kelvin (from WeatherSystem)
    temperature_response_range: f32, // Temperature range for combustion rate normalization (K)
    air_density_kg_m3: f32,     // Air density (kg/m³)
    atmospheric_mixing_height_m: f32, // Atmospheric mixing height (m)
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(0) var<storage, read> temperature: array<f32>;
@group(0) @binding(1) var<storage, read_write> fuel_load: array<f32>;
@group(0) @binding(2) var<storage, read_write> moisture: array<f32>;
@group(0) @binding(3) var<storage, read_write> oxygen: array<f32>;
@group(0) @binding(4) var<storage, read> level_set: array<f32>;
@group(0) @binding(5) var<storage, read_write> heat_release: array<f32>;
@group(0) @binding(6) var<uniform> params: CombustionParams;

// Constants
const LATENT_HEAT_WATER: f32 = 2260.0;  // kJ/kg
const OXYGEN_STOICHIOMETRIC_RATIO: f32 = 1.33;  // kg O₂/kg fuel
// Specific heat capacity of dry woody biomass (kJ/(kg·K))
// Representative value for wood: range 1.3-2.0 kJ/(kg·K), using mid-range value.
// Matches CPU implementation WOOD_SPECIFIC_HEAT_CAPACITY constant
const WOOD_SPECIFIC_HEAT_CAPACITY: f32 = 1.6;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // Check bounds
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }
    
    let idx = id.y * params.width + id.x;
    
    let t = temperature[idx];
    let f = fuel_load[idx];
    let m = moisture[idx];
    let o2 = oxygen[idx];
    let is_burning = level_set[idx] < 0.0;
    
    // Skip if not burning or no fuel
    if (!is_burning || f < 1e-6) {
        heat_release[idx] = 0.0;
        return;
    }
    
    // Fuel properties from uniform buffer (passed from Rust Fuel type)
    let ignition_temp = params.ignition_temp_k;
    let moisture_extinction = params.moisture_extinction;
    let heat_content_kj = params.heat_content_kj;
    let self_heating_fraction = params.self_heating_fraction;
    let base_burn_rate = params.burn_rate_coefficient;
    
    let cell_area = params.cell_size * params.cell_size;
    let mass = f * cell_area;
    let moisture_mass = m * mass;
    
    // 1. CRITICAL: Moisture evaporation FIRST
    // Moisture must evaporate before temperature rises
    // Use thermodynamically exact formula: Q = m × c_p × ΔT (matches CPU implementation)
    // Ambient temperature from WeatherSystem (via uniform buffer)
    let ambient_temp = params.ambient_temp_k;
    var thermal_energy_kj = 0.0;
    if (t > ambient_temp) {
        // Q = m × c_p × ΔT (fundamental thermodynamics)
        thermal_energy_kj = mass * WOOD_SPECIFIC_HEAT_CAPACITY * (t - ambient_temp);
    }
    
    // Maximum moisture that can evaporate given available thermal energy
    // E_latent = m_water × L_v where L_v = 2260 kJ/kg
    let max_evap = thermal_energy_kj / LATENT_HEAT_WATER;
    let moisture_evaporated = min(moisture_mass, max_evap);
    
    // Update moisture (this happens BEFORE combustion)
    if (moisture_mass > 0.0) {
        moisture[idx] = max(0.0, (moisture_mass - moisture_evaporated) / mass);
    }
    
    // 2. Fuel consumption rate (only if conditions met)
    var burn_rate = 0.0;
    
    // Check ignition conditions
    if (m < moisture_extinction && t > ignition_temp) {
        // Moisture damping factor
        let moisture_damping = 1.0 - (m / moisture_extinction);

        // Temperature factor (normalized) - fuel-specific response range
        let temp_factor = min(1.0, (t - ignition_temp) / params.temperature_response_range);

        // Base burn rate
        burn_rate = base_burn_rate * moisture_damping * temp_factor;
        
        // 3. Oxygen limitation (stoichiometric)
        let o2_required_per_sec = burn_rate * cell_area * OXYGEN_STOICHIOMETRIC_RATIO;
        
        // Available oxygen in cell
        let cell_volume = cell_area * params.atmospheric_mixing_height_m;
        let air_density = params.air_density_kg_m3;
        let o2_available = o2 * air_density * cell_volume;
        
        if (o2_available < o2_required_per_sec * params.dt) {
            // Scale back burn rate based on available oxygen
            burn_rate *= o2_available / (o2_required_per_sec * params.dt);
        }
    }
    
    // 4. Update fuel and oxygen
    let fuel_consumed = min(f, burn_rate * cell_area * params.dt);
    fuel_load[idx] = max(0.0, f - fuel_consumed);
    
    // Oxygen consumed (stoichiometric ratio)
    let o2_consumed = fuel_consumed * OXYGEN_STOICHIOMETRIC_RATIO;
    let cell_volume = cell_area * params.atmospheric_mixing_height_m;
    let air_density = params.air_density_kg_m3;
    let o2_fraction_consumed = o2_consumed / (air_density * cell_volume);
    oxygen[idx] = max(0.0, o2 - o2_fraction_consumed);
    
    // 5. Heat release from combustion
    let heat_released_kj = fuel_consumed * heat_content_kj;
    heat_release[idx] = heat_released_kj * 1000.0 * self_heating_fraction;  // Convert to J
}
