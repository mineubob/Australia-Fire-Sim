// Level Set Evolution Compute Shader
//
// Implements Hamilton-Jacobi evolution equation: ∂φ/∂t + R|∇φ| = 0
// with curvature-dependent spread rate for realistic fire front tracking.
//
// CANONICAL ORDER: This shader applies steps 1-2 of the physics effect sequence:
// 1. Base spread rate (from temperature gradient - computed upstream)
// 2. Slope factor (terrain effect - applied inline in this shader)
//
// Subsequent effects (VLS, valley, crown fire, etc.) are applied in later shader passes.
// See gpu.rs module documentation for complete effect order and rationale.
//
// Physics:
// - Godunov upwind scheme for |∇φ| computation
// - Curvature calculation: κ = (φ_xx φ_y² - 2φ_x φ_y φ_xy + φ_yy φ_x²) / (φ_x² + φ_y²)^(3/2)
// - Curvature-dependent spread: R_eff = R × (1 + κ_coeff × κ)
// - Phase 0: Terrain slope factor (McArthur 1967): ~2× per 10° uphill
// - Stochastic noise for realistic irregularity
//
// References:
// - Sethian (1999) "Level Set Methods and Fast Marching Methods"
// - Margerit & Séro-Guillaume (2002) "Modelling forest fires"
// - McArthur (1967) "Fire behaviour in eucalypt forests"

struct LevelSetParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    curvature_coeff: f32,  // 0.25 per Margerit (2002)
    noise_amplitude: f32,
    time: f32,
    _padding: f32,  // Align to 32 bytes
}

@group(0) @binding(0) var<uniform> params: LevelSetParams;
@group(0) @binding(1) var<storage, read> phi_in: array<f32>;
@group(0) @binding(2) var<storage, read> spread_rate: array<f32>;
@group(0) @binding(3) var<storage, read_write> phi_out: array<f32>;
// Phase 0: Terrain slope and aspect buffers
@group(0) @binding(4) var<storage, read> slope: array<f32>;
@group(0) @binding(5) var<storage, read> aspect: array<f32>;

// Simple hash-based noise function
fn hash_noise(x: f32, y: f32) -> f32 {
    let ix = sin(x * 12.99) * 43758.55;
    let iy = sin(y * 78.23) * 43758.55;
    let fract_val = fract(ix + iy);
    return fract_val * 2.0 - 1.0;  // Range [-1, 1]
}

// Phase 0: Calculate terrain slope factor based on fire spread direction
// Implements McArthur (1967): ~2× per 10° uphill, min 30% downhill
fn calculate_slope_factor(slope_angle: f32, aspect_angle: f32, spread_dir: f32) -> f32 {
    // Calculate upslope direction (aspect points downslope)
    var upslope_dir = aspect_angle + 180.0;
    if (upslope_dir >= 360.0) {
        upslope_dir = upslope_dir - 360.0;
    }
    
    // Angular difference between spread and upslope direction
    var angle_diff = abs(spread_dir - upslope_dir);
    if (angle_diff > 180.0) {
        angle_diff = 360.0 - angle_diff;
    }
    
    // Alignment: 1.0 = spreading directly uphill, -1.0 = downhill, 0 = cross-slope
    let alignment = (180.0 - angle_diff) / 180.0;
    let effective_slope = slope_angle * alignment;
    
    // McArthur slope factor
    if (effective_slope > 0.0) {
        // Uphill: exponential boost (~2× per 10°)
        return 1.0 + pow(effective_slope / 10.0, 1.5) * 2.0;
    } else {
        // Downhill: reduced spread (min 30%)
        return max(1.0 + effective_slope / 30.0, 0.3);
    }
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
    
    // Skip boundary cells (use Dirichlet boundary conditions)
    if (x == 0u || x == params.width - 1u || y == 0u || y == params.height - 1u) {
        phi_out[idx] = phi_in[idx];
        return;
    }
    
    let dx = params.cell_size;
    
    // Get φ and neighbors
    let phi = phi_in[idx];
    let phi_left = phi_in[idx - 1u];
    let phi_right = phi_in[idx + 1u];
    let phi_up = phi_in[idx - params.width];
    let phi_down = phi_in[idx + params.width];
    
    // 1. Compute gradient magnitude using Godunov upwind scheme
    let dx_minus = (phi - phi_left) / dx;
    let dx_plus = (phi_right - phi) / dx;
    let dy_minus = (phi - phi_up) / dx;
    let dy_plus = (phi_down - phi) / dx;
    
    // Godunov Hamiltonian for |∇φ|
    let grad_x = max(max(dx_minus, 0.0), -min(dx_plus, 0.0));
    let grad_y = max(max(dy_minus, 0.0), -min(dy_plus, 0.0));
    let grad_mag = sqrt(grad_x * grad_x + grad_y * grad_y);
    
    // Calculate spread direction from level set gradient (for slope factor)
    let phi_x = (phi_right - phi_left) / (2.0 * dx);
    let phi_y = (phi_down - phi_up) / (2.0 * dx);
    let grad_mag_centered = sqrt(phi_x * phi_x + phi_y * phi_y);
    
    // Spread direction: fire spreads in -∇φ direction (from burned to unburned)
    // Convert to degrees: 0=North, 90=East (atan2(x, y) gives angle from North)
    var spread_direction = 0.0;
    if (grad_mag_centered > 1e-6) {
        let spread_x = -phi_x / grad_mag_centered;
        let spread_y = -phi_y / grad_mag_centered;
        spread_direction = atan2(spread_x, spread_y) * 57.29577951;  // radians to degrees
        if (spread_direction < 0.0) {
            spread_direction = spread_direction + 360.0;
        }
    }
    
    // 2. Compute curvature κ
    let phi_xx = (phi_right - 2.0 * phi + phi_left) / (dx * dx);
    let phi_yy = (phi_down - 2.0 * phi + phi_up) / (dx * dx);
    
    // Get diagonal neighbors for mixed derivative
    let phi_ne = phi_in[idx + params.width + 1u];
    let phi_nw = phi_in[idx + params.width - 1u];
    let phi_se = phi_in[idx - params.width + 1u];
    let phi_sw = phi_in[idx - params.width - 1u];
    
    let phi_xy = (phi_ne - phi_nw - phi_se + phi_sw) / (4.0 * dx * dx);
    
    // Curvature formula: κ = (φ_xx φ_y² - 2φ_x φ_y φ_xy + φ_yy φ_x²) / (φ_x² + φ_y²)^(3/2)
    let grad_sq = phi_x * phi_x + phi_y * phi_y;
    var kappa = 0.0;
    if (grad_sq > 1e-10) {
        let numerator = phi_xx * phi_y * phi_y - 2.0 * phi_x * phi_y * phi_xy + phi_yy * phi_x * phi_x;
        let denom = pow(grad_sq, 1.5);
        kappa = numerator / denom;
    }
    
    // 3. Get spread rate
    let r = spread_rate[idx];
    
    // 4. Apply curvature effect (Margerit 2002)
    // Convex (κ > 0) → faster spread (fingers)
    // Concave (κ < 0) → slower spread (indentations)
    let r_curvature = r * (1.0 + params.curvature_coeff * kappa);
    
    // 5. Phase 0: Apply terrain slope factor (McArthur 1967)
    let slope_angle = slope[idx];
    let aspect_angle = aspect[idx];
    let slope_factor = calculate_slope_factor(slope_angle, aspect_angle, spread_direction);
    let r_slope = r_curvature * slope_factor;
    
    // 6. Add stochastic noise for realistic irregularity
    let noise = hash_noise(f32(x) * 0.05 + params.time * 0.1, f32(y) * 0.05 + params.time * 0.1);
    let r_final = r_slope * (1.0 + params.noise_amplitude * noise);
    
    // 7. Hamilton-Jacobi update: ∂φ/∂t + R|∇φ| = 0
    let dphi = -r_final * grad_mag * params.dt;
    phi_out[idx] = phi + dphi;
}
