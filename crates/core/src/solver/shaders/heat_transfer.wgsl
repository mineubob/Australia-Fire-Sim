// Heat transfer compute shader for GPU
//
// Implements:
// - Stefan-Boltzmann radiation (T⁴ formula, no approximations)
// - Thermal diffusion (Laplacian)
// - Wind advection (upwind scheme)
// - Radiative losses to atmosphere

struct HeatParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    ambient_temp: f32,
    wind_x: f32,
    wind_y: f32,
    stefan_boltzmann: f32,  // 5.67e-8 W/(m²·K⁴)
    // Fuel-specific properties (from Rust Fuel type)
    thermal_diffusivity: f32, // m²/s
    emissivity_burning: f32,  // Flames emissivity (0.9 typical)
    emissivity_unburned: f32, // Fuel bed emissivity (0.7 typical)
    specific_heat_j: f32,     // J/(kg·K)
}

@group(0) @binding(0) var<storage, read> temp_in: array<f32>;
@group(0) @binding(1) var<storage, read> fuel_load: array<f32>;
@group(0) @binding(2) var<storage, read> level_set: array<f32>;
@group(0) @binding(3) var<storage, read_write> temp_out: array<f32>;
@group(0) @binding(4) var<uniform> params: HeatParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // Check bounds
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }
    
    let idx = id.y * params.width + id.x;
    
    // Boundary conditions: Dirichlet (T = T_ambient at edges)
    if (id.x == 0u || id.x == params.width - 1u || id.y == 0u || id.y == params.height - 1u) {
        temp_out[idx] = params.ambient_temp;
        return;
    }
    
    let t = temp_in[idx];
    let f = fuel_load[idx];
    let cell_size_sq = params.cell_size * params.cell_size;
    let mass = f * cell_size_sq;
    
    // Skip cells with negligible fuel
    if (mass < 1e-6) {
        temp_out[idx] = params.ambient_temp;
        return;
    }
    
    // 1. Thermal diffusion (Laplacian)
    let t_left = temp_in[idx - 1u];
    let t_right = temp_in[idx + 1u];
    let t_up = temp_in[idx - params.width];
    let t_down = temp_in[idx + params.width];
    let laplacian = (t_left + t_right + t_up + t_down - 4.0 * t) / cell_size_sq;
    
    // Thermal diffusivity from fuel type (passed via uniform buffer)
    let thermal_diffusivity = params.thermal_diffusivity;
    let diffusion = thermal_diffusivity * laplacian;
    
    // 2. Stefan-Boltzmann radiation exchange with neighbors
    // Emissivity values from fuel type (passed via uniform buffer)
    let is_burning = level_set[idx] < 0.0;
    let emissivity = select(params.emissivity_unburned, params.emissivity_burning, is_burning);
    
    var q_rad = 0.0;
    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            
            let nx = i32(id.x) + dx;
            let ny = i32(id.y) + dy;
            
            // Check bounds
            if (nx < 0 || nx >= i32(params.width) || ny < 0 || ny >= i32(params.height)) {
                continue;
            }
            
            let nidx = u32(ny) * params.width + u32(nx);
            let t_neighbor = temp_in[nidx];
            let dist = sqrt(f32(dx * dx + dy * dy)) * params.cell_size;
            let view_factor = 1.0 / (3.14159 * dist * dist);
            
            // Net radiation: σε(T_n⁴ - T⁴)
            // NEVER simplify - full T⁴ formula
            let t4 = t * t * t * t;
            let tn4 = t_neighbor * t_neighbor * t_neighbor * t_neighbor;
            q_rad += emissivity * params.stefan_boltzmann * (tn4 - t4) * view_factor;
        }
    }
    
    // 3. Radiative loss to atmosphere
    let t4 = t * t * t * t;
    let tamb4 = params.ambient_temp * params.ambient_temp * params.ambient_temp * params.ambient_temp;
    let q_rad_loss = emissivity * params.stefan_boltzmann * (t4 - tamb4);
    
    // 4. Wind advection (upwind scheme)
    var advection = 0.0;
    
    // X-direction
    if (params.wind_x > 0.0 && id.x > 0u) {
        let t_upwind = temp_in[idx - 1u];
        advection += params.wind_x * (t - t_upwind) / params.cell_size;
    } else if (params.wind_x < 0.0 && id.x < params.width - 1u) {
        let t_upwind = temp_in[idx + 1u];
        advection += params.wind_x * (t - t_upwind) / params.cell_size;
    }
    
    // Y-direction
    if (params.wind_y > 0.0 && id.y > 0u) {
        let t_upwind = temp_in[idx - params.width];
        advection += params.wind_y * (t - t_upwind) / params.cell_size;
    } else if (params.wind_y < 0.0 && id.y < params.height - 1u) {
        let t_upwind = temp_in[idx + params.width];
        advection += params.wind_y * (t - t_upwind) / params.cell_size;
    }
    
    // 5. Update temperature
    // Heat capacity: mass × specific_heat
    // Specific heat from fuel type (passed via uniform buffer in J/(kg·K))
    let specific_heat = params.specific_heat_j;
    let heat_capacity = mass * specific_heat;
    
    // Total heat flux (W/m²)
    let dq = params.dt * (diffusion + q_rad - q_rad_loss - advection);
    
    // Temperature change (K)
    let dt_temp = dq / max(heat_capacity, 0.001);
    
    let new_temp = t + dt_temp;
    
    // Clamp to physically reasonable range
    temp_out[idx] = clamp(new_temp, params.ambient_temp - 50.0, 2000.0);
}
