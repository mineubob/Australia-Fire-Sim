// GPU Rothermel Spread Rate Compute Shader with Curvature and Vorticity
// Calculates composite fire spread rate: R(x,y,t) = R_base × wind_factor × slope_factor × (1 + 0.25×κ) × vortex_boost
// References:
// - Rothermel (1972) - USDA INT-115 base spread rate
// - Margerit & Séro-Guillaume (2002) - Curvature effects on fire spread
// - Countryman (1971) - Fire whirl and vorticity physics

// Workgroup size optimized for modern GPUs
@group(0) @binding(0) var<storage, read> phi: array<i32>;              // Level set field (for curvature)
@group(0) @binding(1) var<storage, read> fuel_type_grid: array<u32>;   // Fuel type IDs per cell
@group(0) @binding(2) var<storage, read> wind_field: array<vec2<f32>>; // Wind velocity (u, v) m/s
@group(0) @binding(3) var<storage, read> slope_grid: array<vec2<f32>>; // Terrain slope (dz/dx, dz/dy)
@group(0) @binding(4) var<storage, read> vorticity: array<f32>;        // Vorticity field from wind + plume
@group(0) @binding(5) var<storage, read_write> spread_rate: array<u32>; // Output spread rate (fixed-point)

// Fuel properties (passed as uniform buffer)
struct FuelParams {
    // Per fuel type properties (up to 8 fuel types)
    sigma: array<f32, 8>,              // Surface-area-to-volume ratio (1/ft)
    delta: array<f32, 8>,              // Fuel bed depth (ft)
    mx_dead: array<f32, 8>,            // Dead fuel moisture content
    mx_extinction: array<f32, 8>,      // Moisture of extinction
    heat_content: array<f32, 8>,       // Heat of combustion (BTU/lb)
    fuel_load: array<f32, 8>,          // Fuel load (lb/ft²)
    // Global parameters
    width: u32,
    height: u32,
    dx: f32,                           // Grid spacing (meters)
    fixed_scale: i32,                  // Fixed-point scale
}

@group(0) @binding(6) var<uniform> params: FuelParams;

// Convert fixed-point to float
fn fixed_to_float(val: i32, scale: i32) -> f32 {
    return f32(val) / f32(scale);
}

// Convert float to fixed-point
fn float_to_fixed(val: f32, scale: i32) -> i32 {
    return i32(val * f32(scale));
}

// Get phi value at grid position with bounds checking
fn get_phi(i: i32, j: i32) -> i32 {
    let x = clamp(i, 0, i32(params.width) - 1);
    let y = clamp(j, 0, i32(params.height) - 1);
    let idx = u32(y) * params.width + u32(x);
    return phi[idx];
}

// Calculate curvature κ from level set field
// κ = ∇²φ / |∇φ| (mean curvature of level set)
// Reference: Margerit & Séro-Guillaume (2002) - fire spreads faster on convex fronts
fn calculate_curvature(i: i32, j: i32) -> f32 {
    let phi_c = get_phi(i, j);
    let phi_xp = get_phi(i + 1, j);
    let phi_xm = get_phi(i - 1, j);
    let phi_yp = get_phi(i, j + 1);
    let phi_ym = get_phi(i, j - 1);
    
    // Convert to float for calculation
    let pc = fixed_to_float(phi_c, params.fixed_scale);
    let pxp = fixed_to_float(phi_xp, params.fixed_scale);
    let pxm = fixed_to_float(phi_xm, params.fixed_scale);
    let pyp = fixed_to_float(phi_yp, params.fixed_scale);
    let pym = fixed_to_float(phi_ym, params.fixed_scale);
    
    // First derivatives (central differences)
    let dx_val = params.dx;
    let phi_x = (pxp - pxm) / (2.0 * dx_val);
    let phi_y = (pyp - pym) / (2.0 * dx_val);
    
    // Second derivatives
    let phi_xx = (pxp - 2.0 * pc + pxm) / (dx_val * dx_val);
    let phi_yy = (pyp - 2.0 * pc + pym) / (dx_val * dx_val);
    
    // Gradient magnitude
    let grad_mag = sqrt(phi_x * phi_x + phi_y * phi_y);
    
    if (grad_mag < 1e-6) {
        return 0.0; // No curvature if no gradient
    }
    
    // Mean curvature: κ = ∇²φ / |∇φ|
    let laplacian = phi_xx + phi_yy;
    let curvature = laplacian / grad_mag;
    
    return curvature;
}

// Rothermel (1972) base spread rate calculation
// Reference: USDA Forest Service Research Paper INT-115
fn rothermel_spread_rate(
    fuel_idx: u32,
    wind_speed: f32,
    wind_dir: vec2<f32>,
    slope_vec: vec2<f32>,
) -> f32 {
    // Get fuel properties
    let sigma = params.sigma[fuel_idx];
    let delta = params.delta[fuel_idx];
    let mx = params.mx_dead[fuel_idx];
    let mx_ext = params.mx_extinction[fuel_idx];
    let h = params.heat_content[fuel_idx];
    let w_o = params.fuel_load[fuel_idx];
    
    // Moisture damping coefficient
    let eta_m = 1.0 - 2.59 * (mx / mx_ext) + 5.11 * (mx / mx_ext) * (mx / mx_ext) - 3.52 * (mx / mx_ext) * (mx / mx_ext) * (mx / mx_ext);
    let eta_m_clamped = max(0.0, eta_m);
    
    // Packing ratio (assume optimum for fuel type)
    let beta = 0.01; // Typical value, should be calculated from fuel density
    
    // Reaction intensity (BTU/ft²/min)
    let gamma_prime = sigma * sigma / (495.0 + 0.0594 * sigma * sigma);
    let ir = gamma_prime * w_o * h * eta_m_clamped;
    
    // Wind coefficient (convert m/s to ft/min)
    let wind_speed_ftpm = wind_speed * 196.85; // m/s to ft/min
    let b = 0.02526 * pow(sigma, 0.54);
    let c = 7.47 * exp(-0.133 * pow(sigma, 0.55));
    let e = 0.715 * exp(-3.59e-4 * sigma);
    let phi_w = c * pow(wind_speed_ftpm, b) * pow(beta / 0.0189, -e);
    
    // Slope coefficient
    let slope_mag = sqrt(slope_vec.x * slope_vec.x + slope_vec.y * slope_vec.y);
    let tan_slope = slope_mag;
    let phi_s = 5.275 * pow(beta, -0.3) * tan_slope * tan_slope;
    
    // Propagating flux ratio
    let xi = exp((0.792 + 0.681 * sqrt(sigma)) * (beta + 0.1)) / (192.0 + 0.2595 * sigma);
    
    // Base spread rate (ft/min)
    let r_base = ir * xi * (1.0 + phi_w + phi_s) / (0.3 * 12.0 * w_o); // 0.3 is bulk density, 12 converts to ft
    
    // Convert to m/s
    let r_ms = r_base * 0.00508; // ft/min to m/s
    
    return max(0.0, r_ms);
}

// Vorticity boost factor
// Reference: Countryman (1971) - fire whirls increase spread rate
// Strong vorticity (>0.1 s⁻¹) can increase spread by 50-200%
fn vorticity_boost(vorticity_val: f32) -> f32 {
    // Vorticity threshold for significant effect (0.05 s⁻¹)
    let threshold = 0.05;
    
    if (abs(vorticity_val) < threshold) {
        return 1.0; // No boost
    }
    
    // Boost factor increases with vorticity magnitude
    // Max boost of 2.0× at very high vorticity (0.2 s⁻¹)
    let normalized_vorticity = abs(vorticity_val) / 0.2;
    let boost = 1.0 + min(1.0, normalized_vorticity);
    
    return boost;
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = i32(global_id.x);
    let j = i32(global_id.y);
    
    // Bounds check
    if (global_id.x >= params.width || global_id.y >= params.height) {
        return;
    }
    
    let idx = global_id.y * params.width + global_id.x;
    
    // Get fuel type
    let fuel_type = fuel_type_grid[idx];
    if (fuel_type >= 8u) {
        spread_rate[idx] = 0u; // Invalid fuel type
        return;
    }
    
    // Get wind at this cell
    let wind = wind_field[idx];
    let wind_speed = sqrt(wind.x * wind.x + wind.y * wind.y);
    let wind_dir = normalize(wind);
    
    // Get slope
    let slope = slope_grid[idx];
    
    // Calculate base Rothermel spread rate
    let r_base = rothermel_spread_rate(fuel_type, wind_speed, wind_dir, slope);
    
    // Calculate curvature correction
    let kappa = calculate_curvature(i, j);
    let curvature_factor = 1.0 + 0.25 * kappa; // Margerit & Séro-Guillaume (2002)
    let curvature_factor_clamped = max(0.5, min(2.0, curvature_factor)); // Limit to [0.5, 2.0]
    
    // Calculate vorticity boost
    let vort = vorticity[idx];
    let vortex_factor = vorticity_boost(vort);
    
    // Composite spread rate
    let r_composite = r_base * curvature_factor_clamped * vortex_factor;
    
    // Convert to fixed-point and store
    spread_rate[idx] = u32(max(0.0, r_composite * f32(params.fixed_scale)));
}
