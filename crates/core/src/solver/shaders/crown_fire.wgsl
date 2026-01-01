// Crown Fire Transition and Spread Rate Enhancement Shader
//
// Implements Van Wagner (1977) crown fire initiation criterion and
// Cruz et al. (2005) Australian crown fire spread rates.
//
// Physics:
// - Fire intensity: I = R × W × H (Byram's intensity)
// - Crown ignition: Van Wagner I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
// - Active crown fire: ROS ≥ 3.0 / CBD (Van Wagner critical ROS)
// - Crown spread rate: Cruz et al. (2005) R_crown = 11.02 × U^0.90 × (1 - 0.95 × e^(-0.17 × M))
//
// References:
// - Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire"
// - Cruz, M.G., Alexander, M.E., Wakimoto, R.H. (2005). Crown fire spread in conifers

// Crown fire state enum (matches Rust CrownFireState)
const STATE_SURFACE: u32 = 0u;
const STATE_PASSIVE: u32 = 1u;
const STATE_ACTIVE: u32 = 2u;

struct CrownFireParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    // Canopy properties (CanopyProperties struct)
    canopy_base_height: f32,     // Height to live crown base (m)
    canopy_bulk_density: f32,    // Mass per volume (kg/m³)
    foliar_moisture: f32,        // Live fuel moisture (%)
    canopy_cover_fraction: f32,  // Horizontal cover (0-1)
    canopy_fuel_load: f32,       // Available canopy fuel (kg/m²)
    canopy_heat_content: f32,    // Energy per mass (kJ/kg)
    // Weather parameters
    wind_speed_10m_kmh: f32,     // Wind speed at 10m (km/h)
    // Surface fuel properties
    surface_heat_content: f32,   // Surface fuel heat (kJ/kg), typically 18000
    _padding: f32,               // Align to 16-byte boundary
}

@group(0) @binding(0) var<uniform> params: CrownFireParams;
@group(0) @binding(1) var<storage, read> level_set: array<f32>;
@group(0) @binding(2) var<storage, read> spread_rate_in: array<f32>;
@group(0) @binding(3) var<storage, read> fuel_load: array<f32>;
@group(0) @binding(4) var<storage, read> moisture: array<f32>;
@group(0) @binding(5) var<storage, read_write> spread_rate_out: array<f32>;
@group(0) @binding(6) var<storage, read_write> crown_state: array<u32>;
@group(0) @binding(7) var<storage, read_write> fire_intensity: array<f32>;

// Van Wagner (1977) critical intensity for crown ignition
// I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
fn critical_intensity(cbh: f32, fmc: f32) -> f32 {
    let base_term = 0.010 * cbh * (460.0 + 25.9 * fmc);
    return pow(base_term, 1.5);
}

// Van Wagner (1977) critical ROS for active crown fire
// R_critical = 3.0 / CBD (in m/min)
fn critical_ros_m_min(cbd: f32) -> f32 {
    if (cbd <= 0.0) {
        return 1e10;  // Infinity replacement
    }
    return 3.0 / cbd;
}

// Cruz et al. (2005) crown fire spread rate
// R_crown = 11.02 × U₁₀^0.90 × (1 - 0.95 × e^(-0.17 × M_dead)) (m/min)
fn crown_spread_rate_m_s(wind_kmh: f32, dead_moisture_frac: f32) -> f32 {
    let wind = max(wind_kmh, 0.0);
    let moisture_pct = max(dead_moisture_frac * 100.0, 0.0);
    
    let wind_term = pow(wind, 0.90);
    let moisture_term = 1.0 - 0.95 * exp(-0.17 * moisture_pct);
    
    let ros_m_min = 11.02 * wind_term * moisture_term;
    return ros_m_min / 60.0;  // Convert to m/s
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    
    // Boundary check
    if (x >= params.width || y >= params.height) {
        return;
    }
    
    let idx = y * params.width + x;
    let phi = level_set[idx];
    let surface_ros = spread_rate_in[idx];
    let fuel = fuel_load[idx];
    let moist = moisture[idx];
    
    // Only process burning cells (phi < 0)
    if (phi >= 0.0 || surface_ros <= 0.0) {
        // Not burning - surface fire only
        spread_rate_out[idx] = surface_ros;
        crown_state[idx] = STATE_SURFACE;
        fire_intensity[idx] = 0.0;
        return;
    }
    
    // Calculate surface fire intensity: I = H × W × R (Byram's formula)
    // H in kJ/kg, W in kg/m², R in m/s → I in kW/m
    let surface_intensity = params.surface_heat_content * fuel * surface_ros;
    fire_intensity[idx] = surface_intensity;
    
    // Calculate critical intensity for crown ignition
    let i_critical = critical_intensity(params.canopy_base_height, params.foliar_moisture);
    
    // Check if intensity is sufficient for crown ignition
    if (surface_intensity < i_critical) {
        // Below threshold - remains surface fire
        spread_rate_out[idx] = surface_ros;
        crown_state[idx] = STATE_SURFACE;
        return;
    }
    
    // Crown ignition criteria met - check for active vs passive
    let surface_ros_m_min = surface_ros * 60.0;
    let r_critical = critical_ros_m_min(params.canopy_bulk_density);
    
    if (surface_ros_m_min >= r_critical) {
        // Active crown fire - use crown spread rate
        let crown_ros = crown_spread_rate_m_s(params.wind_speed_10m_kmh, moist);
        
        // Use the higher of surface or crown ROS
        let effective_ros = max(surface_ros, crown_ros);
        spread_rate_out[idx] = effective_ros;
        crown_state[idx] = STATE_ACTIVE;
        
        // Update intensity to include crown fuel
        let crown_intensity = params.canopy_heat_content * params.canopy_fuel_load * crown_ros;
        fire_intensity[idx] = surface_intensity + crown_intensity;
    } else {
        // Passive crown fire (torching) - ~1.5x surface spread enhancement
        let passive_factor = 1.0 + 0.5 * params.canopy_cover_fraction;
        spread_rate_out[idx] = surface_ros * passive_factor;
        crown_state[idx] = STATE_PASSIVE;
    }
}
