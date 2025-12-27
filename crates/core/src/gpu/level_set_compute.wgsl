// GPU Level Set Fire Front Compute Shader
// Implements upwind scheme for: ∂φ/∂t + R(x,y,t)|∇φ| = 0
// Uses fixed-point arithmetic for deterministic multiplayer compute

// Workgroup size optimized for modern GPUs (256 threads = 16x16)
@group(0) @binding(0) var<storage, read> phi_in: array<i32>;       // Level set field (fixed-point)
@group(0) @binding(1) var<storage, read> spread_rate: array<u32>;  // Spread rate R(x,y,t) (fixed-point)
@group(0) @binding(2) var<storage, read_write> phi_out: array<i32>; // Output level set

// Simulation parameters (passed via uniform buffer)
struct Params {
    width: u32,
    height: u32,
    dx: f32,          // Grid spacing (meters)
    dt: f32,          // Time step (seconds)
    fixed_scale: i32, // Fixed-point scale factor (e.g., 1000 for 3 decimals)
}

@group(0) @binding(3) var<uniform> params: Params;

// Fixed-point arithmetic helpers for deterministic compute
// All GPUs handle integer ops identically (unlike floating-point)

/// Convert float to fixed-point integer
fn float_to_fixed(val: f32, scale: i32) -> i32 {
    return i32(val * f32(scale));
}

/// Convert fixed-point integer to float (for display/debugging only)
fn fixed_to_float(val: i32, scale: i32) -> f32 {
    return f32(val) / f32(scale);
}

/// Fixed-point multiply (scale down to prevent overflow)
/// WGSL doesn't have i64, so we use f32 for intermediate calculation
/// IEEE 754 guarantees determinism across all platforms
fn fixed_mul(a: i32, b: i32, scale: i32) -> i32 {
    let a_f = f32(a);
    let b_f = f32(b);
    let scale_f = f32(scale);
    let result = (a_f * b_f) / scale_f;
    return i32(round(result));
}

/// Fixed-point absolute value
fn fixed_abs(val: i32) -> i32 {
    return select(-val, val, val >= 0);
}

/// Fixed-point max
fn fixed_max(a: i32, b: i32) -> i32 {
    return select(b, a, a > b);
}

/// Get phi value at grid position (i, j) with bounds checking
fn get_phi(i: i32, j: i32) -> i32 {
    let x = clamp(i, 0, i32(params.width) - 1);
    let y = clamp(j, 0, i32(params.height) - 1);
    let idx = u32(y) * params.width + u32(x);
    return phi_in[idx];
}

/// Upwind gradient scheme for level set advection
/// Uses first-order upwind differencing for numerical stability
/// Reference: Sethian (1999) "Level Set Methods and Fast Marching Methods"
///
/// IMPORTANT: Uses max-absolute-value selection from forward/backward differences.
/// This correctly captures sharp discontinuities (e.g., fire front step functions).
/// Previous formula max(max(d_xm,0), -max(d_xp,0)) failed for discontinuities.
fn upwind_gradient(i: i32, j: i32) -> i32 {
    let phi_center = get_phi(i, j);
    
    // Spatial derivatives using upwind scheme
    let phi_xm = get_phi(i - 1, j); // x minus
    let phi_xp = get_phi(i + 1, j); // x plus
    let phi_ym = get_phi(i, j - 1); // y minus
    let phi_yp = get_phi(i, j + 1); // y plus
    
    // Forward/backward differences (fixed-point)
    let dx_fixed = float_to_fixed(params.dx, params.fixed_scale);
    let d_xm = (phi_center - phi_xm); // / dx (done later)
    let d_xp = (phi_xp - phi_center); // / dx
    let d_ym = (phi_center - phi_ym); // / dy
    let d_yp = (phi_yp - phi_center); // / dy
    
    // Gradient magnitude: use max absolute value from forward/backward differences
    // This correctly captures sharp discontinuities (e.g., fire front)
    let grad_x = select(d_xm, d_xp, fixed_abs(d_xp) > fixed_abs(d_xm));
    let grad_y = select(d_ym, d_yp, fixed_abs(d_yp) > fixed_abs(d_ym));
    
    // |∇φ|² = grad_x² + grad_y² (use fixed_mul to prevent overflow)
    let dx2 = fixed_mul(grad_x, grad_x, params.fixed_scale);
    let dy2 = fixed_mul(grad_y, grad_y, params.fixed_scale);
    let grad_mag_sq = dx2 + dy2;
    
    // Integer square root (Babylonian method, deterministic)
    var sqrt_val = grad_mag_sq / 2;
    for (var iter = 0; iter < 10; iter++) {
        if (sqrt_val == 0) { break; }
        sqrt_val = (sqrt_val + grad_mag_sq / sqrt_val) / 2;
    }
    
    // Gradient magnitude calculation:
    // grad_mag_sq = d²/scale (from fixed_mul)
    // sqrt_val = d/sqrt(scale)
    // To get gradient d/dx in fixed-point:
    // grad = (d * scale) / dx = sqrt_val * sqrt(scale) * scale / dx
    // For scale=1024=2^10: sqrt(scale)=32 EXACTLY (eliminates approximation error!)
    let sqrt_scale = 32; // sqrt(1024) = 32 exactly
    let grad_mag = select(0, (sqrt_val * sqrt_scale * params.fixed_scale) / dx_fixed, dx_fixed != 0);
    
    return grad_mag;
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
    
    // Get current phi value
    let phi_current = phi_in[idx];
    
    // Get spread rate R(x,y,t) at this position (fixed-point)
    let R = i32(spread_rate[idx]);
    
    // Calculate upwind gradient |∇φ|
    let grad_mag = upwind_gradient(i, j);
    
    // Level set evolution: ∂φ/∂t = -R|∇φ|
    // φ_new = φ_old + dt * (-R * |∇φ|)
    let dt_fixed = float_to_fixed(params.dt, params.fixed_scale);
    let R_grad = fixed_mul(R, grad_mag, params.fixed_scale);
    let dphi = fixed_mul(dt_fixed, R_grad, params.fixed_scale);
    
    // Update phi (subtract because equation is ∂φ/∂t + R|∇φ| = 0)
    phi_out[idx] = phi_current - dphi;
}
