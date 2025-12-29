//! Vertical heat transfer between fuel layers.
//!
//! Implements Stefan-Boltzmann radiative transfer and convective heat
//! flux between burning lower layers and unburned upper layers.
//!
//! # Physics Implementation
//!
//! Heat flows upward from burning fuel through two mechanisms:
//!
//! 1. **Radiative transfer**: Stefan-Boltzmann law with view factor geometry
//!    ```text
//!    Q_rad = ε × σ × (T_source^4 - T_target^4) × F_view
//!    ```
//!
//! 2. **Convective transfer**: Flame plume heating
//!    ```text
//!    Q_conv = h × (T_flame - T_target)
//!    ```
//!
//! # Critical Rule: Moisture Evaporation FIRST
//!
//! Per project requirements, moisture must evaporate (consuming 2.26 MJ/kg
//! latent heat) BEFORE fuel temperature rises. This prevents unrealistic
//! temperature spikes in wet fuel.
//!
//! # Scientific References
//!
//! - "Fundamentals of Heat and Mass Transfer" (Incropera et al., 2002)
//! - "An Introduction to Fire Dynamics" (Drysdale, 2011)
//! - "Wildland Fire Spread by Radiation" (Albini, 1986)

use super::fuel_layers::{FuelLayer, LayerState};

/// Stefan-Boltzmann constant (W/m²K⁴)
///
/// Universal physical constant for blackbody radiation.
pub const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Latent heat of water vaporization (J/kg)
///
/// Heat absorbed when water changes from liquid to vapor.
/// This heat is consumed BEFORE temperature can rise.
pub const LATENT_HEAT_WATER: f32 = 2_260_000.0;

/// Parameters for vertical heat flux calculation.
///
/// Groups the environmental and temporal parameters needed for
/// calculating heat flux between fuel layers.
#[derive(Clone, Debug)]
pub struct FluxParams {
    /// Flame height in meters (from Byram's equation)
    pub flame_height_m: f32,

    /// Canopy cover fraction (0-1), affects radiative view factor
    pub canopy_cover_fraction: f32,

    /// Timestep in seconds
    pub dt_seconds: f32,
}

impl FluxParams {
    /// Create new flux parameters.
    #[must_use]
    pub fn new(flame_height_m: f32, canopy_cover_fraction: f32, dt_seconds: f32) -> Self {
        Self {
            flame_height_m,
            canopy_cover_fraction: canopy_cover_fraction.clamp(0.0, 1.0),
            dt_seconds,
        }
    }
}

/// Vertical heat transfer calculator.
///
/// Computes radiative and convective heat flux between
/// vertically adjacent fuel layers during fire propagation.
#[derive(Clone, Debug)]
pub struct VerticalHeatTransfer {
    /// Effective emissivity for vegetation (typically 0.9)
    ///
    /// Combined emissivity accounting for fuel bed properties.
    /// Flames have emissivity ~0.95, fuel beds ~0.7-0.9.
    pub emissivity: f32,

    /// Base convective heat transfer coefficient (W/m²K)
    ///
    /// Modified by wind and flame geometry during calculations.
    pub convective_coeff_base: f32,
}

impl Default for VerticalHeatTransfer {
    fn default() -> Self {
        Self {
            emissivity: 0.9,
            convective_coeff_base: 25.0, // Typical for natural convection with fire plume
        }
    }
}

impl VerticalHeatTransfer {
    /// Create a new heat transfer calculator with specified parameters.
    ///
    /// # Arguments
    /// * `emissivity` - Effective emissivity (0-1)
    /// * `convective_coeff_base` - Base convective coefficient (W/m²K)
    #[must_use]
    pub fn new(emissivity: f32, convective_coeff_base: f32) -> Self {
        Self {
            emissivity: emissivity.clamp(0.0, 1.0),
            convective_coeff_base: convective_coeff_base.max(0.0),
        }
    }

    /// Calculate heat flux from source layer to target layer (J/m² per timestep).
    ///
    /// Combines radiative and convective heat transfer mechanisms.
    /// Heat only flows upward (from lower to higher layers).
    ///
    /// # Physics
    ///
    /// **Radiative transfer (Stefan-Boltzmann):**
    /// ```text
    /// Q_rad = ε × σ × (T_source^4 - T_target^4) × F_view
    /// ```
    ///
    /// **Convective transfer:**
    /// ```text
    /// Q_conv = h × (T_flame - T_target) × F_plume
    /// ```
    ///
    /// Where:
    /// - `F_view`: View factor based on layer geometry
    /// - `F_plume`: Fraction of flame plume reaching target layer
    ///
    /// # Arguments
    /// * `source` - Source layer state (burning layer)
    /// * `source_layer` - Which layer the source is
    /// * `target` - Target layer state (receiving heat)
    /// * `target_layer` - Which layer the target is
    /// * `params` - Environmental and temporal flux parameters
    ///
    /// # Returns
    /// Heat flux in J/m² for this timestep. Returns 0 if:
    /// - Source is not burning
    /// - Target is above source (no downward heat transfer)
    /// - Flame doesn't reach target layer
    #[must_use]
    pub fn calculate_flux(
        &self,
        source: &LayerState,
        source_layer: FuelLayer,
        target: &LayerState,
        target_layer: FuelLayer,
        params: &FluxParams,
    ) -> f32 {
        // No flux if source is not burning
        if !source.burning {
            return 0.0;
        }

        // Heat only flows upward: source must be below target
        let source_height = source_layer.representative_height();
        let target_height = target_layer.representative_height();

        if source_height >= target_height {
            // No downward heat transfer
            return 0.0;
        }

        let vertical_distance = target_height - source_height;

        // Check if flame reaches target layer
        // Flame extends from source height upward
        let flame_top = source_height + params.flame_height_m;
        if flame_top < target_layer.height_range().0 {
            // Flame doesn't reach target layer - reduced but not zero flux
            // Still receive radiation from hot gases
            let distance_beyond_flame = target_layer.height_range().0 - flame_top;
            let attenuation = (-0.5 * distance_beyond_flame).exp();
            if attenuation < 0.01 {
                return 0.0;
            }
        }

        // 1. Radiative heat flux (Stefan-Boltzmann)
        // Q_rad = ε × σ × (T_source^4 - T_target^4) × F_view
        // NEVER simplify - use full T^4 formula as per project rules
        let t_source = source.temperature;
        let t_target = target.temperature;

        // View factor decreases with distance squared and canopy cover
        // F_view = (1 - canopy_cover) / (π × d²)
        // Normalized to unit area exchange
        let view_factor = (1.0 - params.canopy_cover_fraction)
            / (std::f32::consts::PI * vertical_distance * vertical_distance);
        let view_factor = view_factor.min(1.0); // Cap at unity

        let q_rad = self.emissivity
            * STEFAN_BOLTZMANN
            * (t_source.powi(4) - t_target.powi(4))
            * view_factor;

        // 2. Convective heat flux (flame plume)
        // Plume fraction: how much of the flame plume reaches the target
        let plume_fraction = if params.flame_height_m > 0.0 {
            (1.0 - vertical_distance / (params.flame_height_m * 2.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Flame temperature typically 800-1200°C above ambient
        // Use source temperature directly as it represents flame temperature
        let q_conv = self.convective_coeff_base * (t_source - t_target) * plume_fraction;

        // Total flux (W/m²) × dt = J/m²
        let total_flux_w = (q_rad + q_conv).max(0.0); // No negative flux (cooling handled separately)
        total_flux_w * params.dt_seconds
    }

    /// Apply heat to layer, handling moisture evaporation FIRST (2.26 MJ/kg).
    ///
    /// This is a critical physics requirement: moisture must evaporate
    /// (absorbing latent heat) before fuel temperature can rise.
    ///
    /// # Physics
    ///
    /// 1. Calculate water mass from moisture fraction
    /// 2. Calculate heat needed to evaporate all water: `m_water × L_v`
    /// 3. If heat received < heat needed: evaporate what we can, no temp rise
    /// 4. If heat received > heat needed: evaporate all, remaining heat raises temp
    ///
    /// # Arguments
    /// * `layer` - Layer state to update (mutated in place)
    /// * `heat_received_j_m2` - Heat received in J/m²
    /// * `fuel_heat_capacity_j_kg_k` - Heat capacity of fuel in J/(kg·K)
    pub fn apply_heat_to_layer(
        layer: &mut LayerState,
        heat_received_j_m2: f32,
        fuel_heat_capacity_j_kg_k: f32,
    ) {
        if heat_received_j_m2 <= 0.0 || layer.fuel_load <= 0.0 {
            return;
        }

        // 1. Calculate water mass per unit area
        // moisture is fraction (0-1) of fuel mass that is water
        let water_mass_per_m2 = layer.moisture * layer.fuel_load;

        // 2. Calculate heat needed to evaporate all water
        let heat_for_evaporation = water_mass_per_m2 * LATENT_HEAT_WATER;

        // 3. Apply heat: evaporation FIRST
        if heat_received_j_m2 <= heat_for_evaporation && heat_for_evaporation > 0.0 {
            // All heat goes to evaporation, no temperature rise
            let water_evaporated = heat_received_j_m2 / LATENT_HEAT_WATER;
            let remaining_water = (water_mass_per_m2 - water_evaporated).max(0.0);
            layer.moisture = remaining_water / layer.fuel_load;
        } else {
            // All water evaporates, remaining heat raises temperature
            layer.moisture = 0.0;
            let remaining_heat = heat_received_j_m2 - heat_for_evaporation;

            // Temperature rise: ΔT = Q / (m × c)
            let thermal_mass = layer.fuel_load * fuel_heat_capacity_j_kg_k;
            if thermal_mass > 0.0 {
                let delta_t = remaining_heat / thermal_mass;
                layer.temperature += delta_t;

                // Physical limit: can't exceed flame temperature (~1500K)
                layer.temperature = layer.temperature.min(1500.0);
            }
        }

        // Clear accumulated heat
        layer.heat_received = 0.0;
    }

    /// Calculate flame height from fire intensity using Byram's equation.
    ///
    /// ```text
    /// L = 0.0775 × I^0.46
    /// ```
    ///
    /// Where:
    /// - L: Flame length (m)
    /// - I: Byram's fireline intensity (kW/m)
    ///
    /// # Arguments
    /// * `intensity_kw_m` - Fireline intensity in kW/m
    ///
    /// # Returns
    /// Flame height in meters
    #[must_use]
    pub fn flame_height_byram(intensity_kw_m: f32) -> f32 {
        if intensity_kw_m <= 0.0 {
            return 0.0;
        }
        0.0775 * intensity_kw_m.powf(0.46)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_heat_flux_upward_only() {
        let transfer = VerticalHeatTransfer::default();

        let mut surface = LayerState::new(1.0, 0.1);
        surface.burning = true;
        surface.temperature = 1000.0; // Hot burning fuel

        let shrub = LayerState::new(1.0, 0.1);

        // Upward transfer: Surface -> Shrub (should be positive)
        let params = FluxParams::new(3.0, 0.0, 1.0); // 3m flame, no cover, 1s
        let flux_up = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &shrub,
            FuelLayer::Shrub,
            &params,
        );
        assert!(flux_up > 0.0, "Upward heat flux should be positive");

        // Downward transfer: Shrub -> Surface (should be zero)
        let mut shrub_burning = LayerState::new(1.0, 0.1);
        shrub_burning.burning = true;
        shrub_burning.temperature = 1000.0;

        let flux_down = transfer.calculate_flux(
            &shrub_burning,
            FuelLayer::Shrub,
            &surface,
            FuelLayer::Surface,
            &params,
        );
        assert!(
            (flux_down - 0.0).abs() < f32::EPSILON,
            "Downward heat flux should be zero, got {flux_down}",
        );
    }

    #[test]
    fn vertical_heat_flux_zero_when_not_burning() {
        let transfer = VerticalHeatTransfer::default();

        // Source is NOT burning
        let surface = LayerState::new(1.0, 0.1);
        let shrub = LayerState::new(1.0, 0.1);

        let params = FluxParams::new(3.0, 0.0, 1.0);
        let flux = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &shrub,
            FuelLayer::Shrub,
            &params,
        );

        assert!(
            (flux - 0.0).abs() < f32::EPSILON,
            "Heat flux should be zero when source is not burning"
        );
    }

    #[test]
    fn moisture_evaporates_before_temperature_rise() {
        // Test the critical physics requirement: moisture evaporates first

        // Create layer with known fuel load and moisture
        let mut layer = LayerState::new(1.0, 0.5); // 1 kg/m² fuel, 50% moisture
        layer.temperature = 300.0; // Initial temperature

        // Water mass = 0.5 × 1.0 = 0.5 kg/m²
        // Heat to evaporate all = 0.5 × 2,260,000 = 1,130,000 J/m²
        let heat_to_evaporate_all = 0.5 * LATENT_HEAT_WATER;

        // Apply LESS heat than needed to evaporate all water
        let partial_heat = heat_to_evaporate_all / 2.0; // Half the needed heat
        VerticalHeatTransfer::apply_heat_to_layer(
            &mut layer,
            partial_heat,
            2000.0, // Heat capacity
        );

        // Temperature should NOT have changed (all heat went to evaporation)
        assert!(
            (layer.temperature - 300.0).abs() < 0.01,
            "Temperature should not rise while moisture remains. Got: {}",
            layer.temperature
        );

        // Moisture should have decreased
        assert!(
            layer.moisture < 0.5,
            "Moisture should have decreased. Got: {}",
            layer.moisture
        );

        // Moisture should be approximately half (half the heat applied)
        let expected_moisture = 0.25; // Started at 0.5, half evaporated
        assert!(
            (layer.moisture - expected_moisture).abs() < 0.01,
            "Moisture should be ~{}, got: {}",
            expected_moisture,
            layer.moisture
        );
    }

    #[test]
    fn temperature_rises_after_moisture_gone() {
        // Once moisture is gone, remaining heat should raise temperature

        let mut layer = LayerState::new(1.0, 0.1); // 1 kg/m² fuel, 10% moisture
        layer.temperature = 300.0;

        // Water mass = 0.1 × 1.0 = 0.1 kg/m²
        // Heat to evaporate all = 0.1 × 2,260,000 = 226,000 J/m²
        let heat_to_evaporate_all = 0.1 * LATENT_HEAT_WATER;

        // Apply MORE heat than needed to evaporate all water
        let extra_heat = 100_000.0; // Extra heat for temperature rise
        let total_heat = heat_to_evaporate_all + extra_heat;

        let heat_capacity = 2000.0; // J/(kg·K)
        VerticalHeatTransfer::apply_heat_to_layer(&mut layer, total_heat, heat_capacity);

        // Moisture should be zero
        assert!(
            layer.moisture.abs() < f32::EPSILON,
            "All moisture should have evaporated. Got: {}",
            layer.moisture
        );

        // Temperature should have risen
        // ΔT = Q / (m × c) = 100,000 / (1.0 × 2000) = 50 K
        let expected_temp = 300.0 + (extra_heat / (1.0 * heat_capacity));
        assert!(
            (layer.temperature - expected_temp).abs() < 0.1,
            "Temperature should be ~{}, got: {}",
            expected_temp,
            layer.temperature
        );
    }

    #[test]
    fn latent_heat_value_correct() {
        // Verify constant matches physics requirement: 2.26 MJ/kg = 2,260,000 J/kg
        assert!(
            (LATENT_HEAT_WATER - 2_260_000.0).abs() < 1.0,
            "Latent heat should be 2,260,000 J/kg"
        );
    }

    #[test]
    fn stefan_boltzmann_constant_correct() {
        // Verify Stefan-Boltzmann constant
        assert!(
            (STEFAN_BOLTZMANN - 5.67e-8).abs() < 1e-10,
            "Stefan-Boltzmann constant should be 5.67e-8 W/(m²K⁴)"
        );
    }

    #[test]
    fn flame_height_byram() {
        // Test Byram's flame height equation: L = 0.0775 × I^0.46

        // Zero intensity = zero flame height
        assert!((VerticalHeatTransfer::flame_height_byram(0.0) - 0.0).abs() < f32::EPSILON);

        // Known value test: I = 1000 kW/m
        // L = 0.0775 × 1000^0.46 = 0.0775 × 23.99 ≈ 1.86 m
        let flame_1000 = VerticalHeatTransfer::flame_height_byram(1000.0);
        assert!(
            (flame_1000 - 1.86).abs() < 0.1,
            "Flame height at 1000 kW/m should be ~1.86m, got: {flame_1000}",
        );

        // Higher intensity = higher flame
        let flame_5000 = VerticalHeatTransfer::flame_height_byram(5000.0);
        assert!(
            flame_5000 > flame_1000,
            "Higher intensity should produce taller flame"
        );
    }

    #[test]
    fn canopy_cover_reduces_flux() {
        let transfer = VerticalHeatTransfer::default();

        let mut surface = LayerState::new(1.0, 0.0);
        surface.burning = true;
        surface.temperature = 1000.0;

        let canopy = LayerState::new(1.0, 0.1);

        // No canopy cover
        let params_open = FluxParams::new(10.0, 0.0, 1.0);
        let flux_open = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &canopy,
            FuelLayer::Canopy,
            &params_open,
        );

        // Full canopy cover
        let params_closed = FluxParams::new(10.0, 1.0, 1.0);
        let flux_closed = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &canopy,
            FuelLayer::Canopy,
            &params_closed,
        );

        assert!(
            flux_open > flux_closed,
            "Open canopy should have higher flux ({flux_open}) than closed ({flux_closed})",
        );
    }

    #[test]
    fn flux_scales_with_timestep() {
        let transfer = VerticalHeatTransfer::default();

        let mut surface = LayerState::new(1.0, 0.0);
        surface.burning = true;
        surface.temperature = 1000.0;

        let shrub = LayerState::new(1.0, 0.1);

        let params_1s = FluxParams::new(3.0, 0.0, 1.0);
        let flux_1s = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &shrub,
            FuelLayer::Shrub,
            &params_1s,
        );

        let params_2s = FluxParams::new(3.0, 0.0, 2.0);
        let flux_2s = transfer.calculate_flux(
            &surface,
            FuelLayer::Surface,
            &shrub,
            FuelLayer::Shrub,
            &params_2s,
        );

        // Flux should scale linearly with timestep
        assert!(
            (flux_2s - 2.0 * flux_1s).abs() < 0.01,
            "Flux should scale linearly with timestep"
        );
    }
}
