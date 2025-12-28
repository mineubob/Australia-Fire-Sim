// Signed Distance Reinitialization Compute Shader
//
// Periodically reinitializes the level set field to maintain the signed distance
// property |∇φ| ≈ 1. This prevents numerical diffusion and maintains accuracy.
//
// Implements: ∂φ/∂τ = sign(φ₀)(1 - |∇φ|)
//
// This PDE is iterated for a few pseudo-time steps to restore the signed distance
// property while preserving the zero level set (fire front location).
//
// References:
// - Sussman, Smereka & Osher (1994) "A Level Set Approach for Computing Solutions 
//   to Incompressible Two-Phase Flow"
// - Sethian (1999) "Level Set Methods and Fast Marching Methods"

struct ReinitParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt_pseudo: f32,  // Pseudo-timestep for reinitialization (typically 0.5 * cell_size)
    _padding: vec3<f32>,  // Align to 32 bytes
}

@group(0) @binding(0) var<uniform> params: ReinitParams;
@group(0) @binding(1) var<storage, read> phi_in: array<f32>;
@group(0) @binding(2) var<storage, read> phi_original: array<f32>;  // φ₀ - preserve sign
@group(0) @binding(3) var<storage, read_write> phi_out: array<f32>;

// Sign function with smoothing near zero to avoid instability
fn sign_smooth(phi: f32, epsilon: f32) -> f32 {
    return phi / sqrt(phi * phi + epsilon * epsilon);
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
    
    // Skip boundary cells
    if (x == 0u || x == params.width - 1u || y == 0u || y == params.height - 1u) {
        phi_out[idx] = phi_in[idx];
        return;
    }
    
    let dx = params.cell_size;
    let phi = phi_in[idx];
    
    // Get neighbors
    let phi_left = phi_in[idx - 1u];
    let phi_right = phi_in[idx + 1u];
    let phi_up = phi_in[idx - params.width];
    let phi_down = phi_in[idx + params.width];
    
    // Compute upwind gradient components
    let dx_minus = (phi - phi_left) / dx;
    let dx_plus = (phi_right - phi) / dx;
    let dy_minus = (phi - phi_up) / dx;
    let dy_plus = (phi_down - phi) / dx;
    
    // Upwind scheme based on sign of φ₀
    let phi0 = phi_original[idx];
    let s = sign_smooth(phi0, dx);  // Smoothed sign function
    
    var grad_mag_sq = 0.0;
    
    if (s > 0.0) {
        // For positive φ, use backward differences where positive, forward where negative
        let gx = max(max(dx_minus, 0.0), -min(dx_plus, 0.0));
        let gy = max(max(dy_minus, 0.0), -min(dy_plus, 0.0));
        grad_mag_sq = gx * gx + gy * gy;
    } else {
        // For negative φ, use forward differences where positive, backward where negative
        let gx = max(-min(dx_minus, 0.0), max(dx_plus, 0.0));
        let gy = max(-min(dy_minus, 0.0), max(dy_plus, 0.0));
        grad_mag_sq = gx * gx + gy * gy;
    }
    
    let grad_mag = sqrt(grad_mag_sq);
    
    // Reinitialization equation: ∂φ/∂τ = sign(φ₀)(1 - |∇φ|)
    let dphi = s * (1.0 - grad_mag) * params.dt_pseudo;
    phi_out[idx] = phi + dphi;
}
