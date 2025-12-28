//! Level set fire front tracking module
//!
//! Implements level set φ field evolution for tracking fire fronts with realistic
//! jagged perimeters. Uses curvature-dependent spread rates to create fingers and
//! indentations characteristic of real wildfire behavior.
//!
//! # Level Set Method
//!
//! The fire front is represented implicitly as the zero level set of φ:
//! - φ < 0: Burned region
//! - φ = 0: Fire front
//! - φ > 0: Unburned region
//!
//! # Physics Implementation
//!
//! Evolution equation: ∂φ/∂t + R|∇φ| = 0
//!
//! Where:
//! - `R` is the spread rate (from heat flux)
//! - |∇φ| is computed using Godunov upwind scheme
//! - `R` is modified by curvature: `R_eff` = `R` × (1 + `κ_coeff` × κ)
//!
//! # Curvature Effect (Margerit 2002)
//!
//! - Convex regions (κ > 0): Faster spread → fingers
//! - Concave regions (κ < 0): Slower spread → indentations
//! - Coefficient: `κ_coeff` = 0.25 for realistic fire shapes

/// Curvature coefficient for spread rate modification
/// Based on Margerit & Séro-Guillaume (2002)
pub const CURVATURE_COEFFICIENT: f32 = 0.25;

/// Physics parameters for level set evolution
#[derive(Debug, Clone, Copy)]
pub struct LevelSetParams {
    /// Timestep in seconds
    pub dt: f32,
    /// Cell size in meters
    pub cell_size: f32,
    /// Curvature coefficient (default 0.25)
    pub curvature_coeff: f32,
    /// Noise amplitude for stochastic variation (0.0-0.2)
    pub noise_amplitude: f32,
    /// Current simulation time (for noise)
    pub time: f32,
}

impl Default for LevelSetParams {
    fn default() -> Self {
        Self {
            dt: 0.1,
            cell_size: 10.0,
            curvature_coeff: CURVATURE_COEFFICIENT,
            noise_amplitude: 0.05,
            time: 0.0,
        }
    }
}

/// CPU implementation of level set evolution
///
/// Evolves the level set field φ using:
/// - Godunov upwind scheme for gradient magnitude
/// - Curvature calculation for spread rate modification
/// - Hamilton-Jacobi evolution equation
///
/// # Arguments
///
/// * `phi_in` - Input level set field (signed distance)
/// * `phi_out` - Output level set field (updated)
/// * `spread_rate` - Spread rate field (m/s) from heat transfer
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `params` - Physics parameters
///
/// # References
///
/// - Sethian (1999) "Level Set Methods and Fast Marching Methods"
/// - Margerit & Séro-Guillaume (2002) "Modelling forest fires"
#[allow(clippy::too_many_arguments)]
pub fn step_level_set_cpu(
    phi_in: &[f32],
    phi_out: &mut [f32],
    spread_rate: &[f32],
    width: usize,
    height: usize,
    params: LevelSetParams,
) {
    let dx = params.cell_size;

    for y in 0..height {
        for x in 0..width {
            // Skip boundary cells
            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                let idx = y * width + x;
                phi_out[idx] = phi_in[idx];
                continue;
            }

            let idx = y * width + x;
            let phi = phi_in[idx];

            // Get neighbors
            let phi_left = phi_in[idx - 1];
            let phi_right = phi_in[idx + 1];
            let phi_up = phi_in[idx - width];
            let phi_down = phi_in[idx + width];

            // 1. Compute gradient magnitude using Godunov upwind scheme
            let dx_minus = (phi - phi_left) / dx;
            let dx_plus = (phi_right - phi) / dx;
            let dy_minus = (phi - phi_up) / dx;
            let dy_plus = (phi_down - phi) / dx;

            // Godunov Hamiltonian for |∇φ|
            let grad_x = f32::max(f32::max(dx_minus, 0.0), -f32::min(dx_plus, 0.0));
            let grad_y = f32::max(f32::max(dy_minus, 0.0), -f32::min(dy_plus, 0.0));
            let grad_mag = f32::sqrt(grad_x * grad_x + grad_y * grad_y);

            // 2. Compute curvature κ
            let phi_xx = (phi_right - 2.0 * phi + phi_left) / (dx * dx);
            let phi_yy = (phi_down - 2.0 * phi + phi_up) / (dx * dx);

            // Get diagonal neighbors for mixed derivative
            let phi_ne = if x < width - 1 && y < height - 1 {
                phi_in[idx + width + 1]
            } else {
                phi
            };
            let phi_nw = if x > 0 && y < height - 1 {
                phi_in[idx + width - 1]
            } else {
                phi
            };
            let phi_se = if x < width - 1 && y > 0 {
                phi_in[idx - width + 1]
            } else {
                phi
            };
            let phi_sw = if x > 0 && y > 0 {
                phi_in[idx - width - 1]
            } else {
                phi
            };

            let phi_xy = (phi_ne - phi_nw - phi_se + phi_sw) / (4.0 * dx * dx);
            let phi_x = (phi_right - phi_left) / (2.0 * dx);
            let phi_y = (phi_down - phi_up) / (2.0 * dx);

            // Curvature formula: κ = (φ_xx φ_y² - 2φ_x φ_y φ_xy + φ_yy φ_x²) / (φ_x² + φ_y²)^(3/2)
            let grad_sq = phi_x * phi_x + phi_y * phi_y;
            let kappa = if grad_sq > 1e-10 {
                let numerator =
                    phi_xx * phi_y * phi_y - 2.0 * phi_x * phi_y * phi_xy + phi_yy * phi_x * phi_x;
                let denom = grad_sq.powf(1.5);
                numerator / denom
            } else {
                0.0
            };

            // 3. Get spread rate
            let r = spread_rate[idx];

            // 4. Apply curvature effect (Margerit 2002)
            // Convex (κ > 0) → faster spread (fingers)
            // Concave (κ < 0) → slower spread (indentations)
            let r_effective = r * (1.0 + params.curvature_coeff * kappa);

            // 5. Add simple noise for stochastic variation
            // Simple deterministic noise based on position and time
            #[expect(clippy::cast_precision_loss)]
            let noise = simple_noise(
                x as f32 * 0.05 + params.time * 0.1,
                y as f32 * 0.05 + params.time * 0.1,
            );
            let r_final = r_effective * (1.0 + params.noise_amplitude * noise);

            // 6. Hamilton-Jacobi update: ∂φ/∂t + R|∇φ| = 0
            let dphi = -r_final * grad_mag * params.dt;
            phi_out[idx] = phi + dphi;
        }
    }
}

/// Simple noise function for stochastic variation
///
/// Uses a simple hash-based noise to avoid dependency on complex noise libraries
fn simple_noise(x: f32, y: f32) -> f32 {
    let ix = (x * 12.99).sin() * 43758.55;
    let iy = (y * 78.23).sin() * 43758.55;
    let fract = (ix + iy).fract();
    fract * 2.0 - 1.0 // Range [-1, 1]
}

/// Compute spread rate from temperature gradient
///
/// Spread rate emerges from heat physics rather than being prescribed.
/// Based on heat flux required to bring adjacent fuel to ignition.
///
/// # Arguments
///
/// * `temperature` - Temperature field (Kelvin)
/// * `fuel_load` - Fuel load per cell (kg/m²)
/// * `moisture` - Moisture fraction (0-1)
/// * `spread_rate_out` - Output spread rate field (m/s)
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `cell_size` - Cell size in meters
#[allow(clippy::too_many_arguments)]
pub fn compute_spread_rate_cpu(
    temperature: &[f32],
    fuel_load: &[f32],
    moisture: &[f32],
    spread_rate_out: &mut [f32],
    width: usize,
    height: usize,
    cell_size: f32,
) {
    // Fuel properties (should come from fuel lookup table)
    let ignition_temp = 573.15; // ~300°C (K)
    let specific_heat = 2000.0; // J/(kg·K)
    let latent_heat_water = 2260000.0; // J/kg

    for idx in 0..width * height {
        let temperature_val = temperature[idx];
        let fuel_load_val = fuel_load[idx];
        let moisture_val = moisture[idx];

        // Skip if no fuel or already at ignition
        if fuel_load_val < 1e-6 || temperature_val >= ignition_temp {
            spread_rate_out[idx] = 0.0;
            continue;
        }

        // Estimate heat flux from temperature gradient to neighbors
        // Simplified: assume heat flows from hot to cold
        let x = idx % width;
        let y = idx / width;

        let mut max_temp_diff = 0.0_f32;

        // Check neighbors
        if x > 0 {
            max_temp_diff = max_temp_diff.max(temperature[idx - 1] - temperature_val);
        }
        if x < width - 1 {
            max_temp_diff = max_temp_diff.max(temperature[idx + 1] - temperature_val);
        }
        if y > 0 {
            max_temp_diff = max_temp_diff.max(temperature[idx - width] - temperature_val);
        }
        if y < height - 1 {
            max_temp_diff = max_temp_diff.max(temperature[idx + width] - temperature_val);
        }

        // Estimate heat flux (W/m²) using thermal conductivity
        let thermal_conductivity = 0.1; // W/(m·K) for fuel bed
        let heat_flux = thermal_conductivity * max_temp_diff / cell_size;

        // Heat required to ignite this cell
        let mass_per_area = fuel_load_val; // kg/m²
        let sensible_heat = mass_per_area * specific_heat * (ignition_temp - temperature_val);
        let latent_heat = moisture_val * mass_per_area * latent_heat_water;
        let total_heat_required = sensible_heat + latent_heat;

        // Spread rate: distance per time = heat_flux / (heat_per_area)
        // Units: (W/m²) / (J/m²) = 1/s → multiply by cell_size to get m/s
        let spread_rate = if total_heat_required > 0.0 {
            (heat_flux * cell_size) / total_heat_required
        } else {
            0.0
        };

        spread_rate_out[idx] = spread_rate.clamp(0.0, 10.0); // Clamp to reasonable range
    }
}

/// Reinitialize level set field to maintain signed distance property
///
/// Solves ∂φ/∂τ = sign(φ₀)(1 - |∇φ|) to restore |∇φ| ≈ 1.
/// This prevents numerical diffusion and maintains accuracy.
///
/// # Arguments
///
/// * `phi_in` - Input level set field
/// * `phi_out` - Output reinitialized field
/// * `phi_original` - Original field (preserves sign and zero level set location)
/// * `width` - Grid width
/// * `height` - Grid height
/// * `cell_size` - Cell size in meters
/// * `dt_pseudo` - Pseudo-timestep (typically 0.5 × `cell_size`)
///
/// # References
///
/// Synchronize level set with temperature field for ignition
///
/// Updates φ to include cells that have reached ignition temperature.
/// Cells are ignited if:
/// - Currently unburned (φ > 0)
/// - At ignition temperature
/// - Below moisture extinction
/// - Adjacent to burning cell (φ < 0)
///
/// # Arguments
///
/// * `phi` - Level set field (modified in place)
/// * `temperature` - Temperature field (Kelvin)
/// * `moisture` - Moisture fraction (0-1)
/// * `width` - Grid width
/// * `height` - Grid height
/// * `cell_size` - Cell size in meters
/// * `ignition_temp` - Ignition temperature (K)
/// * `moisture_extinction` - Moisture level that prevents burning
#[allow(clippy::too_many_arguments)]
pub fn step_ignition_sync_cpu(
    phi: &mut [f32],
    temperature: &[f32],
    moisture: &[f32],
    width: usize,
    height: usize,
    cell_size: f32,
    ignition_temp: f32,
    moisture_extinction: f32,
) {
    // Create a copy to read from (avoid read-after-write issues)
    let phi_in = phi.to_vec();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Skip boundary cells
            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                continue;
            }

            let phi_val = phi_in[idx];
            let temp = temperature[idx];
            let moist = moisture[idx];

            // Check if currently unburned but at ignition conditions
            if phi_val > 0.0 && temp >= ignition_temp && moist < moisture_extinction {
                // Check if adjacent to burning cell
                let phi_left = phi_in[idx - 1];
                let phi_right = phi_in[idx + 1];
                let phi_up = phi_in[idx - width];
                let phi_down = phi_in[idx + width];

                let has_burning_neighbor =
                    phi_left < 0.0 || phi_right < 0.0 || phi_up < 0.0 || phi_down < 0.0;

                if has_burning_neighbor {
                    // Ignite this cell
                    phi[idx] = -cell_size * 0.5;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_fire_uniform_expansion() {
        // Create a small grid with circular fire
        let width = 21;
        let height = 21;
        let size = width * height;

        let mut phi = vec![f32::MAX; size];
        let mut phi_out = vec![0.0; size];
        let spread_rate = vec![1.0; size]; // Uniform spread rate

        // Initialize circular fire in center
        let cx = width / 2;
        let cy = height / 2;
        let initial_radius = 3.0;

        #[expect(
            clippy::cast_precision_loss,
            reason = "Test uses small grid (<100 cells), no precision loss for f32 mantissa (23 bits)"
        )]
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                phi[idx] = dist - initial_radius;
            }
        }

        let params = LevelSetParams {
            dt: 0.1,
            cell_size: 1.0,
            curvature_coeff: 0.0, // No curvature effect for uniform test
            noise_amplitude: 0.0, // No noise
            time: 0.0,
        };

        // Run one step
        step_level_set_cpu(&phi, &mut phi_out, &spread_rate, width, height, params);

        // Fire should expand uniformly
        // Check a point just outside the initial fire front (should become negative)
        let test_x = cx + 4; // Just outside initial radius of 3
        let test_idx = cy * width + test_x;

        // Initial φ should be positive (unburned)
        assert!(phi[test_idx] > 0.0, "Test point should start unburned");

        // After evolution, φ should decrease (fire advancing)
        assert!(
            phi_out[test_idx] < phi[test_idx],
            "Fire front should advance (φ should decrease from {} to {})",
            phi[test_idx],
            phi_out[test_idx]
        );
    }

    #[test]
    fn test_curvature_sign() {
        // Test that curvature calculation works
        // This is a simplified smoke test
        let width = 11;
        let height = 11;
        let size = width * height;

        let mut phi = vec![0.0; size];
        let mut phi_out = vec![0.0; size];
        let spread_rate = vec![0.1; size];

        // Create a simple gradient
        #[expect(
            clippy::cast_precision_loss,
            reason = "Test uses small grid (<100 cells), no precision loss for f32 mantissa (23 bits)"
        )]
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                phi[idx] = x as f32;
            }
        }

        let params = LevelSetParams {
            dt: 0.001, // Small timestep
            cell_size: 1.0,
            curvature_coeff: 0.25,
            noise_amplitude: 0.0,
            time: 0.0,
        };

        step_level_set_cpu(&phi, &mut phi_out, &spread_rate, width, height, params);

        // Should have some evolution
        let center_idx = 5 * width + 5;
        assert!(
            (phi_out[center_idx] - phi[center_idx]).abs() > 0.0,
            "Level set should evolve"
        );
    }

    #[test]
    fn test_godunov_upwind_scheme() {
        // Test that Godunov scheme correctly computes gradient magnitude
        let width = 5;
        let height = 5;
        let size = width * height;

        let mut phi = vec![0.0; size];
        let mut phi_out = vec![0.0; size];
        let spread_rate = vec![1.0; size];

        // Create a linear gradient: φ = x (increasing to the right)
        #[expect(
            clippy::cast_precision_loss,
            reason = "Test uses small grid (<100 cells), no precision loss for f32 mantissa (23 bits)"
        )]
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                phi[idx] = x as f32;
            }
        }

        let params = LevelSetParams {
            dt: 0.1,
            cell_size: 1.0,
            curvature_coeff: 0.0,
            noise_amplitude: 0.0,
            time: 0.0,
        };

        step_level_set_cpu(&phi, &mut phi_out, &spread_rate, width, height, params);

        // For linear gradient, |∇φ| should be constant = 1.0
        // Evolution should be uniform
        // Check that interior cells evolved
        let center_idx = 2 * width + 2;
        assert!(
            phi_out[center_idx] != phi[center_idx],
            "Interior cells should evolve"
        );
    }

    #[test]
    fn test_spread_rate_computation() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let mut temperature = vec![293.15; size]; // Ambient
        let fuel_load = vec![1.0; size]; // 1 kg/m²
        let moisture = vec![0.1; size]; // 10%
        let mut spread_rate = vec![0.0; size];

        // Create hot spot in center
        let center_idx = 2 * width + 2;
        temperature[center_idx] = 800.0; // Very hot

        compute_spread_rate_cpu(
            &temperature,
            &fuel_load,
            &moisture,
            &mut spread_rate,
            width,
            height,
            10.0,
        );

        // Cells adjacent to hot spot should have non-zero spread rate
        let left_idx = center_idx - 1;
        assert!(
            spread_rate[left_idx] > 0.0,
            "Adjacent cell should have positive spread rate: {}",
            spread_rate[left_idx]
        );
    }

    #[test]
    fn test_ignition_sync_updates_phi() {
        // Test that ignition sync correctly updates φ when conditions are met
        let width = 5;
        let height = 5;
        let size = width * height;

        let mut phi = vec![f32::MAX; size]; // All unburned
        let mut temperature = vec![293.15; size]; // Ambient
        let moisture = vec![0.1; size]; // 10% moisture

        // Create a burning cell in center
        let center_idx = 2 * width + 2;
        phi[center_idx] = -1.0; // Burning

        // Heat up adjacent cell to ignition temperature
        let right_idx = center_idx + 1;
        temperature[right_idx] = 600.0; // Above ignition (573.15 K)

        step_ignition_sync_cpu(
            &mut phi,
            &temperature,
            &moisture,
            width,
            height,
            1.0,    // cell_size
            573.15, // ignition_temp
            0.3,    // moisture_extinction
        );

        // Right cell should now be ignited (negative φ)
        assert!(
            phi[right_idx] < 0.0,
            "Cell should be ignited when hot and adjacent to fire: {}",
            phi[right_idx]
        );
    }

    #[test]
    fn test_ignition_sync_respects_moisture() {
        // Test that high moisture prevents ignition
        let width = 5;
        let height = 5;
        let size = width * height;

        let mut phi = vec![f32::MAX; size];
        let mut temperature = vec![293.15; size];
        let mut moisture = vec![0.1; size];

        // Burning center
        let center_idx = 2 * width + 2;
        phi[center_idx] = -1.0;

        // Hot adjacent cell but too wet
        let right_idx = center_idx + 1;
        temperature[right_idx] = 600.0; // Hot enough
        moisture[right_idx] = 0.4; // Too wet (> 0.3 extinction)

        step_ignition_sync_cpu(
            &mut phi,
            &temperature,
            &moisture,
            width,
            height,
            1.0,
            573.15,
            0.3,
        );

        // Should NOT be ignited
        assert!(
            phi[right_idx] > 0.0,
            "Cell should not ignite when moisture is too high"
        );
    }
}
