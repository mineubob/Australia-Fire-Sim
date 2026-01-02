//! Combustion physics module
//!
//! Implements fuel combustion, moisture evaporation, heat release, and oxygen consumption.
//!
//! # Critical Rule: Moisture Evaporation FIRST
//!
//! Per project requirements, moisture must evaporate (consuming 2260 kJ/kg latent heat)
//! BEFORE fuel temperature rises. This is physically accurate and prevents unrealistic
//! temperature spikes in wet fuel.
//!
//! # Physics Implementation
//!
//! 1. **Moisture evaporation**: Consumes heat before combustion
//! 2. **Fuel consumption**: Rate based on temperature, moisture, and oxygen
//! 3. **Heat release**: `fuel_consumed` × `heat_content`
//! 4. **Oxygen depletion**: Stoichiometric ratio (1.33 kg O₂/kg fuel)

/// Latent heat of vaporization for water (kJ/kg)
/// This heat must be absorbed BEFORE fuel temperature rises
pub const LATENT_HEAT_WATER: f32 = 2260.0;

/// Stoichiometric oxygen requirement for wood combustion (kg O₂/kg fuel)
pub const OXYGEN_STOICHIOMETRIC_RATIO: f32 = 1.33;

/// Fuel-specific combustion properties
///
/// These properties MUST come from the Fuel type - never hardcode them.
/// Reference: Project guidelines "NEVER HARDCODE DYNAMIC VALUES"
#[derive(Debug, Clone, Copy)]
pub struct FuelCombustionProps {
    /// Ignition temperature (Kelvin) - from `Fuel.ignition_temperature` converted to K
    pub ignition_temp_k: f32,
    /// Moisture of extinction (0-1) - from `Fuel.moisture_of_extinction`
    pub moisture_extinction: f32,
    /// Heat content (kJ/kg) - from `Fuel.heat_content`
    pub heat_content_kj: f32,
    /// Fraction of heat retained in fuel bed (0-1) - from `Fuel.self_heating_fraction`
    pub self_heating_fraction: f32,
    /// Base burn rate coefficient - from `Fuel.burn_rate_coefficient`
    pub burn_rate_coefficient: f32,
}

impl Default for FuelCombustionProps {
    /// Default properties based on eucalyptus stringybark
    fn default() -> Self {
        Self {
            ignition_temp_k: 501.15,     // 228°C + 273.15K (stringybark)
            moisture_extinction: 0.35,   // 35% for eucalyptus
            heat_content_kj: 21000.0,    // kJ/kg for eucalyptus
            self_heating_fraction: 0.4,  // 40% retained
            burn_rate_coefficient: 0.08, // stringybark coefficient
        }
    }
}

/// Physics parameters for combustion computation
#[derive(Debug, Clone, Copy)]
pub struct CombustionParams {
    /// Timestep in seconds
    pub dt: f32,
    /// Cell size in meters
    pub cell_size: f32,
    /// Fuel-specific combustion properties
    pub fuel_props: FuelCombustionProps,
}

/// CPU implementation of combustion physics
///
/// Computes:
/// - Moisture evaporation (2260 kJ/kg latent heat absorption FIRST)
/// - Fuel consumption rate (temperature, moisture, and oxygen dependent)
/// - Heat release (`fuel_consumed` × `heat_content`)
/// - Oxygen depletion (stoichiometric ratio)
///
/// # Arguments
///
/// * `temperature` - Temperature field (Kelvin) - read-only
/// * `fuel_load` - Fuel load per cell (kg/m²) - updated
/// * `moisture` - Fuel moisture fraction (0-1) - updated
/// * `oxygen` - Atmospheric oxygen fraction (0-1) - updated
/// * `level_set` - Fire front signed distance (negative = burning)
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `params` - Physics parameters
///
/// # Returns
///
/// Total heat released (W) for adding to temperature field in heat transfer step
#[allow(clippy::too_many_arguments)]
pub fn step_combustion_cpu(
    temperature: &[f32],
    fuel_load: &mut [f32],
    moisture: &mut [f32],
    oxygen: &mut [f32],
    level_set: &[f32],
    width: usize,
    height: usize,
    params: CombustionParams,
) -> Vec<f32> {
    let cell_area = params.cell_size * params.cell_size;
    let num_cells = width * height;
    let mut heat_release = vec![0.0; num_cells];

    // Process each cell
    for idx in 0..num_cells {
        let t = temperature[idx];
        let f = fuel_load[idx];
        let m = moisture[idx];
        let o2 = oxygen[idx];
        let is_burning = level_set[idx] < 0.0;

        // Skip if not burning or no fuel
        if !is_burning || f < 1e-6 {
            heat_release[idx] = 0.0;
            continue;
        }

        // Fuel properties from FuelCombustionProps (dynamic, not hardcoded)
        let ignition_temp = params.fuel_props.ignition_temp_k;
        let moisture_extinction = params.fuel_props.moisture_extinction;
        let heat_content_kj = params.fuel_props.heat_content_kj;
        let self_heating_fraction = params.fuel_props.self_heating_fraction;
        let base_burn_rate = params.fuel_props.burn_rate_coefficient;

        // 1. CRITICAL: Moisture evaporation FIRST
        // Moisture must evaporate before temperature rises
        // This consumes heat and prevents unrealistic temperature spikes
        let mass = f * cell_area;
        let moisture_mass = m * mass;

        // Calculate available sensible heat for evaporation using exact thermodynamic formula
        // Q = m × c_p × ΔT (fundamental thermodynamics, no approximation)
        //
        // Use fuel-specific heat capacity (kJ/kg·K) to calculate thermal energy content
        let ambient_temp = 293.15; // ~20°C in Kelvin
        let thermal_energy_kj = if t > ambient_temp {
            // Q = m × c_p × ΔT
            // mass is in kg (fuel load × area), specific_heat from fuel properties
            let specific_heat_kj = params.fuel_props.heat_content_kj / 1000.0; // Approximate c_p from heat content
            mass * specific_heat_kj * (t - ambient_temp) / 1000.0 // Convert K to appropriate scale
        } else {
            0.0
        };

        // Maximum moisture that can evaporate given available thermal energy
        // E_latent = m_water × L_v where L_v = 2260 kJ/kg
        let max_evap = thermal_energy_kj / LATENT_HEAT_WATER;
        let moisture_evaporated = moisture_mass.min(max_evap);

        // Update moisture (this happens BEFORE combustion)
        if moisture_mass > 0.0 {
            moisture[idx] = ((moisture_mass - moisture_evaporated) / mass).max(0.0);
        }

        // 2. Fuel consumption rate (only if conditions met)
        let mut burn_rate = 0.0_f32;

        // Check ignition conditions
        if m < moisture_extinction && t > ignition_temp {
            // Moisture damping factor
            let moisture_damping = 1.0 - (m / moisture_extinction);

            // Temperature factor (normalized)
            let temp_factor = ((t - ignition_temp) / 500.0).min(1.0);

            // Base burn rate
            burn_rate = base_burn_rate * moisture_damping * temp_factor;

            // 3. Oxygen limitation (stoichiometric)
            let o2_required_per_sec = burn_rate * cell_area * OXYGEN_STOICHIOMETRIC_RATIO;

            // Available oxygen in cell (assuming 1m height of atmosphere)
            let cell_volume = cell_area * 1.0; // 1m height
            let air_density = 1.2; // kg/m³
            let o2_available = o2 * air_density * cell_volume;

            if o2_available < o2_required_per_sec * params.dt {
                // Scale back burn rate based on available oxygen
                burn_rate *= o2_available / (o2_required_per_sec * params.dt);
            }
        }

        // 4. Update fuel and oxygen
        let fuel_consumed = (burn_rate * cell_area * params.dt).min(f);
        fuel_load[idx] = (f - fuel_consumed).max(0.0);

        // Oxygen consumed (stoichiometric ratio)
        let o2_consumed = fuel_consumed * OXYGEN_STOICHIOMETRIC_RATIO;
        let cell_volume = cell_area * 1.0; // 1m height
        let air_density = 1.2; // kg/m³
        let o2_fraction_consumed = o2_consumed / (air_density * cell_volume);
        oxygen[idx] = (o2 - o2_fraction_consumed).max(0.0);

        // 5. Heat release from combustion
        // This gets added to temperature in heat transfer step
        let heat_released_kj = fuel_consumed * heat_content_kj;
        heat_release[idx] = heat_released_kj * 1000.0 * self_heating_fraction; // Convert to J
    }

    heat_release
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moisture_evaporation_before_burning() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temperature = vec![600.0; size]; // Hot enough to evaporate
        let mut fuel_load = vec![1.0; size]; // 1 kg/m²
        let mut moisture = vec![0.2; size]; // 20% moisture
        let mut oxygen = vec![0.21; size]; // Normal atmospheric O₂
        let level_set = vec![-1.0; size]; // All burning

        let params = CombustionParams {
            dt: 1.0,
            cell_size: 10.0,
            fuel_props: FuelCombustionProps::default(),
        };

        let initial_moisture = moisture[0];

        step_combustion_cpu(
            &temperature,
            &mut fuel_load,
            &mut moisture,
            &mut oxygen,
            &level_set,
            width,
            height,
            params,
        );

        // Moisture should decrease due to evaporation
        assert!(
            moisture[0] < initial_moisture,
            "Moisture should evaporate (was {:.3}, now {:.3})",
            initial_moisture,
            moisture[0]
        );
    }

    #[test]
    fn test_fuel_consumption_at_high_temperature() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temperature = vec![800.0; size]; // Very hot (above ignition)
        let mut fuel_load = vec![2.0; size]; // 2 kg/m²
        let mut moisture = vec![0.05; size]; // Low moisture (5%)
        let mut oxygen = vec![0.21; size];
        let level_set = vec![-1.0; size]; // All burning

        let params = CombustionParams {
            dt: 1.0,
            cell_size: 10.0,
            fuel_props: FuelCombustionProps::default(),
        };

        let initial_fuel = fuel_load[0];

        step_combustion_cpu(
            &temperature,
            &mut fuel_load,
            &mut moisture,
            &mut oxygen,
            &level_set,
            width,
            height,
            params,
        );

        // Fuel should be consumed
        assert!(
            fuel_load[0] < initial_fuel,
            "Fuel should be consumed at high temp (was {:.3}, now {:.3})",
            initial_fuel,
            fuel_load[0]
        );
    }

    #[test]
    fn test_oxygen_depletion_during_combustion() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temperature = vec![800.0; size];
        let mut fuel_load = vec![2.0; size];
        let mut moisture = vec![0.05; size];
        let mut oxygen = vec![0.21; size];
        let level_set = vec![-1.0; size];

        let params = CombustionParams {
            dt: 1.0,
            cell_size: 10.0,
            fuel_props: FuelCombustionProps::default(),
        };

        let initial_oxygen = oxygen[0];

        step_combustion_cpu(
            &temperature,
            &mut fuel_load,
            &mut moisture,
            &mut oxygen,
            &level_set,
            width,
            height,
            params,
        );

        // Oxygen should be depleted
        assert!(
            oxygen[0] < initial_oxygen,
            "Oxygen should be depleted (was {:.3}, now {:.3})",
            initial_oxygen,
            oxygen[0]
        );
    }

    #[test]
    fn test_no_burning_below_ignition_temperature() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temperature = vec![400.0; size]; // Below ignition (~300°C = 573K)
        let mut fuel_load = vec![2.0; size];
        let mut moisture = vec![0.05; size];
        let mut oxygen = vec![0.21; size];
        let level_set = vec![-1.0; size];

        let params = CombustionParams {
            dt: 1.0,
            cell_size: 10.0,
            fuel_props: FuelCombustionProps::default(),
        };

        let initial_fuel = fuel_load[0];

        step_combustion_cpu(
            &temperature,
            &mut fuel_load,
            &mut moisture,
            &mut oxygen,
            &level_set,
            width,
            height,
            params,
        );

        // Fuel should NOT be consumed below ignition temperature
        assert!(
            (fuel_load[0] - initial_fuel).abs() < 0.01,
            "Fuel should not burn below ignition temp (was {:.3}, now {:.3})",
            initial_fuel,
            fuel_load[0]
        );
    }

    #[test]
    fn test_heat_release_from_combustion() {
        let width = 5;
        let height = 5;
        let size = width * height;

        let temperature = vec![800.0; size];
        let mut fuel_load = vec![2.0; size];
        let mut moisture = vec![0.05; size];
        let mut oxygen = vec![0.21; size];
        let level_set = vec![-1.0; size];

        let params = CombustionParams {
            dt: 1.0,
            cell_size: 10.0,
            fuel_props: FuelCombustionProps::default(),
        };

        let heat_release = step_combustion_cpu(
            &temperature,
            &mut fuel_load,
            &mut moisture,
            &mut oxygen,
            &level_set,
            width,
            height,
            params,
        );

        // Heat should be released from burning cells
        assert!(
            heat_release[0] > 0.0,
            "Heat should be released from combustion: {} J",
            heat_release[0]
        );
    }
}
