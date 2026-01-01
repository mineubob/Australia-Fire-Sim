//! Heat transfer physics module
//!
//! Implements Stefan-Boltzmann radiation, thermal diffusion, wind advection,
//! and radiative cooling for both CPU and GPU backends.
//!
//! # Physics Implementation
//!
//! The heat equation with combustion source:
//! ```text
//! ∂T/∂t = α∇²T + Q_combustion - Q_radiation - Q_convection + Q_wind_advection
//! ```
//!
//! Where:
//! - `α∇²T`: Thermal diffusion (conduction through fuel bed)
//! - `Q_combustion`: Heat release from burning fuel
//! - `Q_radiation`: Stefan-Boltzmann radiative losses = εσ(`T⁴` - `T_amb⁴`)
//! - `Q_convection`: Convective heat transfer to atmosphere
//! - `Q_wind_advection`: Wind-driven heat transport

use rayon::prelude::*;

/// Stefan-Boltzmann constant (W/(m²·K⁴))
pub const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Physics parameters for heat transfer computation
#[derive(Debug, Clone, Copy)]
pub struct HeatTransferParams {
    /// Timestep in seconds
    pub dt: f32,
    /// Wind velocity in x direction (m/s)
    pub wind_x: f32,
    /// Wind velocity in y direction (m/s)
    pub wind_y: f32,
    /// Ambient temperature in Kelvin
    pub ambient_temp: f32,
    /// Cell size in meters
    pub cell_size: f32,
    /// Fuel-specific thermal properties
    pub fuel_props: HeatTransferFuelProps,
}

/// Fuel-specific properties for heat transfer calculations
///
/// These properties MUST come from the Fuel type - never hardcode them.
/// Reference: Project guidelines "NEVER HARDCODE DYNAMIC VALUES"
#[derive(Debug, Clone, Copy)]
pub struct HeatTransferFuelProps {
    /// Thermal diffusivity (m²/s) - from `Fuel.thermal_diffusivity`
    pub thermal_diffusivity: f32,
    /// Emissivity for burning fuel (0-1) - typically 0.9 for flames
    pub emissivity_burning: f32,
    /// Emissivity for unburned fuel (0-1) - typically 0.7 for fuel bed
    pub emissivity_unburned: f32,
    /// Specific heat (kJ/(kg·K)) - from `Fuel.specific_heat`
    pub specific_heat_kj: f32,
}

impl Default for HeatTransferFuelProps {
    /// Default properties based on eucalyptus stringybark
    fn default() -> Self {
        Self {
            thermal_diffusivity: 1.5e-7, // m²/s for coarse wood fuel
            emissivity_burning: 0.9,     // Flames have high emissivity
            emissivity_unburned: 0.7,    // Fuel bed has lower emissivity
            specific_heat_kj: 1.5,       // kJ/(kg·K) for eucalyptus
        }
    }
}

/// CPU implementation of heat transfer physics
///
/// Computes:
/// - Thermal diffusion (Laplacian operator)
/// - Stefan-Boltzmann radiation (`T⁴` exchange with neighbors and atmosphere)
/// - Wind advection (upwind finite difference scheme)
/// - Boundary conditions (Dirichlet: `T = T_ambient` at edges)
///
/// # Arguments
///
/// * `temp_in` - Input temperature field (Kelvin)
/// * `temp_out` - Output temperature field (Kelvin)
/// * `level_set` - Fire front signed distance (negative = burning)
/// * `fuel_load` - Fuel load per cell (kg/m²)
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `params` - Physics parameters
///
/// # Physics Accuracy
///
/// - Uses full T⁴ Stefan-Boltzmann formula (no linearization)
/// - View factor geometry: 1/(πr²) for radiative exchange
/// - Upwind scheme for numerical stability in advection
/// - Specific heat from fuel properties
#[allow(clippy::too_many_arguments)]
pub fn step_heat_transfer_cpu(
    temp_in: &[f32],
    temp_out: &mut [f32],
    level_set: &[f32],
    fuel_load: &[f32],
    width: usize,
    height: usize,
    params: HeatTransferParams,
) {
    let cell_size_sq = params.cell_size * params.cell_size;

    // Parallel iteration over grid cells
    temp_out
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, cell_temp) in row.iter_mut().enumerate().take(width) {
                // Boundary conditions: Dirichlet (T = T_ambient at edges)
                if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                    *cell_temp = params.ambient_temp;
                    continue;
                }

                let idx = y * width + x;
                let t = temp_in[idx];
                let mass = fuel_load[idx] * cell_size_sq;

                // Skip cells with negligible fuel
                if mass < 1e-6 {
                    *cell_temp = params.ambient_temp;
                    continue;
                }

                // 1. Thermal diffusion (Laplacian)
                let t_left = temp_in[idx - 1];
                let t_right = temp_in[idx + 1];
                let t_up = temp_in[idx - width];
                let t_down = temp_in[idx + width];
                let laplacian = (t_left + t_right + t_up + t_down - 4.0 * t) / cell_size_sq;

                // Use fuel-specific thermal diffusivity (not hardcoded)
                let thermal_diffusivity = params.fuel_props.thermal_diffusivity;
                let diffusion = thermal_diffusivity * laplacian;

                // 2. Stefan-Boltzmann radiation exchange with neighbors
                // Use fuel-specific emissivity values (not hardcoded)
                let emissivity = if level_set[idx] < 0.0 {
                    params.fuel_props.emissivity_burning
                } else {
                    params.fuel_props.emissivity_unburned
                };

                let mut q_rad = 0.0_f32;
                for dy in -1..=1_i32 {
                    for dx in -1..=1_i32 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }

                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;

                        // Check bounds
                        if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                            continue;
                        }

                        let nidx = (ny as usize) * width + (nx as usize);
                        let t_neighbor = temp_in[nidx];
                        #[expect(clippy::cast_precision_loss)]
                        let dist = f32::sqrt((dx * dx + dy * dy) as f32) * params.cell_size;
                        let view_factor = 1.0 / (std::f32::consts::PI * dist * dist);

                        // Net radiation: σε(T_n⁴ - T⁴)
                        // NEVER simplify - full T⁴ formula as per project rules
                        q_rad += emissivity
                            * STEFAN_BOLTZMANN
                            * (t_neighbor.powi(4) - t.powi(4))
                            * view_factor;
                    }
                }

                // 3. Radiative loss to atmosphere
                // σε(T⁴ - T_amb⁴)
                let q_rad_loss =
                    emissivity * STEFAN_BOLTZMANN * (t.powi(4) - params.ambient_temp.powi(4));

                // 4. Wind advection (upwind finite difference scheme)
                let mut advection = 0.0;

                // X-direction advection
                if params.wind_x > 0.0 && x > 0 {
                    let t_upwind = temp_in[idx - 1];
                    advection += params.wind_x * (t - t_upwind) / params.cell_size;
                } else if params.wind_x < 0.0 && x < width - 1 {
                    let t_upwind = temp_in[idx + 1];
                    advection += params.wind_x * (t - t_upwind) / params.cell_size;
                }

                // Y-direction advection
                if params.wind_y > 0.0 && y > 0 {
                    let t_upwind = temp_in[idx - width];
                    advection += params.wind_y * (t - t_upwind) / params.cell_size;
                } else if params.wind_y < 0.0 && y < height - 1 {
                    let t_upwind = temp_in[idx + width];
                    advection += params.wind_y * (t - t_upwind) / params.cell_size;
                }

                // 5. Update temperature
                // Heat capacity: mass × specific_heat
                // Use fuel-specific specific heat (not hardcoded)
                let specific_heat_kj = params.fuel_props.specific_heat_kj;
                let heat_capacity = mass * specific_heat_kj * 1000.0; // Convert to J/K

                // Total heat flux (W/m²)
                let dq = params.dt * (diffusion + q_rad - q_rad_loss - advection);

                // Temperature change (K)
                let dt_temp = dq / heat_capacity.max(0.001);

                *cell_temp = t + dt_temp;

                // Clamp to physically reasonable range
                // Min: slightly below ambient (cooling)
                // Max: 2000K (typical flame temperatures)
                *cell_temp = cell_temp.clamp(params.ambient_temp - 50.0, 2000.0);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_spot_cools_via_radiation() {
        // Create a simple 10x10 grid
        let width = 10;
        let height = 10;
        let size = width * height;

        let mut temp_in = vec![293.15; size]; // Ambient ~20°C
        let mut temp_out = vec![0.0; size];
        let level_set = vec![f32::MAX; size]; // All unburned
        let fuel_load = vec![1.0; size]; // 1 kg/m²

        // Hot spot in center
        let center = 5 * width + 5;
        temp_in[center] = 600.0; // ~327°C

        let params = HeatTransferParams {
            dt: 1.0,
            wind_x: 0.0,
            wind_y: 0.0,
            ambient_temp: 293.15,
            cell_size: 10.0,
            fuel_props: HeatTransferFuelProps::default(),
        };

        // Run one step
        step_heat_transfer_cpu(
            &temp_in,
            &mut temp_out,
            &level_set,
            &fuel_load,
            width,
            height,
            params,
        );

        // Hot spot should cool down via radiation
        assert!(
            temp_out[center] < temp_in[center],
            "Hot spot should cool down (was {:.2}, now {:.2})",
            temp_in[center],
            temp_out[center]
        );

        // Surrounding cells should warm up from radiation
        let neighbor = center + 1;
        assert!(
            temp_out[neighbor] > temp_in[neighbor],
            "Neighbor should warm up (was {:.2}, now {:.2})",
            temp_in[neighbor],
            temp_out[neighbor]
        );
    }

    #[test]
    fn test_wind_advection_pushes_heat_downwind() {
        let width = 10;
        let height = 10;
        let size = width * height;

        let mut temp_in = vec![293.15; size];
        let mut temp_out = vec![0.0; size];
        let level_set = vec![f32::MAX; size];
        let fuel_load = vec![1.0; size];

        // Hot spot on left side
        let hot_spot = 5 * width + 3;
        temp_in[hot_spot] = 600.0;

        let params = HeatTransferParams {
            dt: 1.0,
            wind_x: 10.0, // 10 m/s wind to the right
            wind_y: 0.0,
            ambient_temp: 293.15,
            cell_size: 10.0,
            fuel_props: HeatTransferFuelProps::default(),
        };

        step_heat_transfer_cpu(
            &temp_in,
            &mut temp_out,
            &level_set,
            &fuel_load,
            width,
            height,
            params,
        );

        // Cell to the right (downwind) should warm more than cell to the left (upwind)
        let right_neighbor = hot_spot + 1;
        let left_neighbor = hot_spot - 1;

        assert!(
            temp_out[right_neighbor] > temp_out[left_neighbor],
            "Downwind cell ({:.2}) should be warmer than upwind cell ({:.2})",
            temp_out[right_neighbor],
            temp_out[left_neighbor]
        );
    }

    #[test]
    fn test_stefan_boltzmann_t4_formula() {
        // Verify that we use full T⁴ formula, not linearized approximation
        let width = 5;
        let height = 5;
        let size = width * height;

        let mut temp_in = vec![293.15; size];
        let mut temp_out = vec![0.0; size];
        let level_set = vec![f32::MAX; size];
        let fuel_load = vec![1.0; size];

        // Two different hot spots to test T⁴ scaling
        let hot1 = 2 * width + 2;
        let hot2 = 2 * width + 3;
        temp_in[hot1] = 500.0; // Moderately hot
        temp_in[hot2] = 1000.0; // Very hot (2x temperature)

        let params = HeatTransferParams {
            dt: 0.1, // Short timestep
            wind_x: 0.0,
            wind_y: 0.0,
            ambient_temp: 293.15,
            cell_size: 10.0,
            fuel_props: HeatTransferFuelProps::default(),
        };

        step_heat_transfer_cpu(
            &temp_in,
            &mut temp_out,
            &level_set,
            &fuel_load,
            width,
            height,
            params,
        );

        // T⁴ scaling: (1000/500)⁴ = 16x more radiative flux
        // So the hotter cell should cool much faster
        let cool_rate_1 = temp_in[hot1] - temp_out[hot1];
        let cool_rate_2 = temp_in[hot2] - temp_out[hot2];

        assert!(
            cool_rate_2 > cool_rate_1 * 10.0,
            "T⁴ scaling: hotter cell should cool much faster (cool_rate_2={cool_rate_2:.4}, cool_rate_1={cool_rate_1:.4})"
        );
    }

    #[test]
    fn test_boundary_conditions_dirichlet() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temp_in = vec![600.0; size]; // All hot
        let mut temp_out = vec![0.0; size];
        let level_set = vec![f32::MAX; size];
        let fuel_load = vec![1.0; size];

        let params = HeatTransferParams {
            dt: 1.0,
            wind_x: 0.0,
            wind_y: 0.0,
            ambient_temp: 293.15,
            cell_size: 10.0,
            fuel_props: HeatTransferFuelProps::default(),
        };

        step_heat_transfer_cpu(
            &temp_in,
            &mut temp_out,
            &level_set,
            &fuel_load,
            width,
            height,
            params,
        );

        // All boundary cells should be set to ambient temperature
        for x in 0..width {
            // Top edge
            assert_eq!(temp_out[x], params.ambient_temp, "Top boundary at x={x}");

            // Bottom edge
            let idx = (height - 1) * width + x;
            assert_eq!(
                temp_out[idx], params.ambient_temp,
                "Bottom boundary at x={x}"
            );
        }

        for y in 1..height - 1 {
            // Left edge
            let idx = y * width;
            assert_eq!(temp_out[idx], params.ambient_temp, "Left boundary at y={y}");

            // Right edge
            let idx = y * width + (width - 1);
            assert_eq!(
                temp_out[idx], params.ambient_temp,
                "Right boundary at y={y}"
            );
        }
    }
}
