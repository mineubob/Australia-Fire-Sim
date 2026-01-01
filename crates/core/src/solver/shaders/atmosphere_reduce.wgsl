// Atmospheric Reduction Shader for PyroCb Calculations
//
// Performs parallel reduction to compute aggregate fire metrics needed for
// atmospheric dynamics and pyroCb formation detection.
//
// Computes:
// - Total fire intensity (sum of all burning cells)
// - Fire centroid (intensity-weighted position)
// - Number of burning cells
// - Maximum fire intensity
//
// Uses workgroup-local reduction with atomics for global accumulation.
//
// References:
// - Fromm et al. (2006) "Pyro-cumulonimbus injection of smoke to the stratosphere"
// - Trentmann et al. (2006) "Modeling of biomass smoke injection"

struct AtmosphereParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
}

struct FireMetrics {
    total_intensity: atomic<u32>,    // Fixed-point sum (×1000)
    weighted_x: atomic<u32>,         // Fixed-point weighted x (×1000)
    weighted_y: atomic<u32>,         // Fixed-point weighted y (×1000)
    cell_count: atomic<u32>,         // Number of burning cells
    max_intensity: atomic<u32>,      // Fixed-point max intensity (×1000)
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

@group(0) @binding(0) var<uniform> params: AtmosphereParams;
@group(0) @binding(1) var<storage, read> level_set: array<f32>;
@group(0) @binding(2) var<storage, read> fire_intensity: array<f32>;
@group(0) @binding(3) var<storage, read_write> metrics: FireMetrics;

// Workgroup shared memory for local reduction
var<workgroup> local_intensity: array<f32, 256>;
var<workgroup> local_weighted_x: array<f32, 256>;
var<workgroup> local_weighted_y: array<f32, 256>;
var<workgroup> local_count: array<u32, 256>;
var<workgroup> local_max: array<f32, 256>;

@compute @workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) local_idx: u32,
) {
    let x = global_id.x;
    let y = global_id.y;
    let local_x = local_id.x;
    let local_y = local_id.y;
    
    // Initialize local memory
    local_intensity[local_idx] = 0.0;
    local_weighted_x[local_idx] = 0.0;
    local_weighted_y[local_idx] = 0.0;
    local_count[local_idx] = 0u;
    local_max[local_idx] = 0.0;
    
    // Process this cell if in bounds
    if (x < params.width && y < params.height) {
        let idx = y * params.width + x;
        let phi = level_set[idx];
        let intensity = fire_intensity[idx];
        
        // Only count burning cells
        if (phi < 0.0 && intensity > 0.0) {
            let world_x = f32(x) * params.cell_size;
            let world_y = f32(y) * params.cell_size;
            
            local_intensity[local_idx] = intensity;
            local_weighted_x[local_idx] = intensity * world_x;
            local_weighted_y[local_idx] = intensity * world_y;
            local_count[local_idx] = 1u;
            local_max[local_idx] = intensity;
        }
    }
    
    workgroupBarrier();
    
    // Parallel reduction within workgroup (256 threads → 1 value)
    // Stride halves each iteration: 128, 64, 32, 16, 8, 4, 2, 1
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (local_idx < stride) {
            local_intensity[local_idx] = local_intensity[local_idx] + local_intensity[local_idx + stride];
            local_weighted_x[local_idx] = local_weighted_x[local_idx] + local_weighted_x[local_idx + stride];
            local_weighted_y[local_idx] = local_weighted_y[local_idx] + local_weighted_y[local_idx + stride];
            local_count[local_idx] = local_count[local_idx] + local_count[local_idx + stride];
            local_max[local_idx] = max(local_max[local_idx], local_max[local_idx + stride]);
        }
        workgroupBarrier();
    }
    
    // First thread in workgroup writes to global using atomics
    if (local_idx == 0u) {
        // Convert to fixed-point for atomic operations (multiply by 1000)
        let intensity_fixed = u32(local_intensity[0] * 1000.0);
        let weighted_x_fixed = u32(local_weighted_x[0] * 1000.0);
        let weighted_y_fixed = u32(local_weighted_y[0] * 1000.0);
        let max_fixed = u32(local_max[0] * 1000.0);
        
        atomicAdd(&metrics.total_intensity, intensity_fixed);
        atomicAdd(&metrics.weighted_x, weighted_x_fixed);
        atomicAdd(&metrics.weighted_y, weighted_y_fixed);
        atomicAdd(&metrics.cell_count, local_count[0]);
        atomicMax(&metrics.max_intensity, max_fixed);
    }
}

// Second pass to clear metrics (run before reduction)
@compute @workgroup_size(1)
fn clear_metrics() {
    atomicStore(&metrics.total_intensity, 0u);
    atomicStore(&metrics.weighted_x, 0u);
    atomicStore(&metrics.weighted_y, 0u);
    atomicStore(&metrics.cell_count, 0u);
    atomicStore(&metrics.max_intensity, 0u);
}
