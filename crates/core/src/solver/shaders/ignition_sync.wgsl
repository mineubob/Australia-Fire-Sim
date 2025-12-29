// Ignition Synchronization Compute Shader
//
// Updates the level set field φ to include cells that have reached ignition
// temperature. This synchronizes the fire front (represented by φ) with the
// temperature field from heat transfer.
//
// Logic:
// - If a cell is unburned (φ > 0) but has reached ignition temperature
// - And has at least one burning neighbor (φ < 0)
// - Then ignite the cell by setting φ to a small negative value
//
// This allows fire to spread through heat transfer physics rather than
// being purely prescribed by spread rate.

struct IgnitionParams {
    width: u32,
    height: u32,
    cell_size: f32,
    ignition_temperature: f32,  // Typically 573.15 K (~300°C)
    moisture_extinction: f32,   // Typically 0.3 (30%)
    _padding: vec3<f32>,  // Align to 32 bytes
}

@group(0) @binding(0) var<uniform> params: IgnitionParams;
@group(0) @binding(1) var<storage, read> phi_in: array<f32>;
@group(0) @binding(2) var<storage, read> temperature: array<f32>;
@group(0) @binding(3) var<storage, read> moisture: array<f32>;
@group(0) @binding(4) var<storage, read_write> phi_out: array<f32>;

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
    
    let phi = phi_in[idx];
    let temp = temperature[idx];
    let moist = moisture[idx];
    
    // Check if currently unburned but at ignition conditions
    if (phi > 0.0 && temp >= params.ignition_temperature && moist < params.moisture_extinction) {
        // Check if adjacent to burning cell (φ < 0)
        let phi_left = phi_in[idx - 1u];
        let phi_right = phi_in[idx + 1u];
        let phi_up = phi_in[idx - params.width];
        let phi_down = phi_in[idx + params.width];
        
        let has_burning_neighbor = 
            phi_left < 0.0 || phi_right < 0.0 || phi_up < 0.0 || phi_down < 0.0;
        
        if (has_burning_neighbor) {
            // Ignite this cell - set to small negative value
            phi_out[idx] = -params.cell_size * 0.5;
        } else {
            // No change
            phi_out[idx] = phi;
        }
    } else {
        // No change - either already burning, too cold, or too wet
        phi_out[idx] = phi;
    }
}
