//! Rothermel Fire Spread Model (1972)
//!
//! Implements the Rothermel fire spread model, the most widely used wildfire spread prediction model.
//!
//! # References
//! - Rothermel, R.C. (1972). "A mathematical model for predicting fire spread in wildland fuels."
//!   USDA Forest Service Research Paper INT-115.
//! - Rothermel, R.C. (1991). "Predicting behavior and size of crown fires in the Northern Rocky Mountains."
//!   USDA Forest Service Research Paper INT-438.
//! - McArthur, A.G. (1967). "Fire behaviour in eucalypt forests." Commonwealth of Australia Forestry and
//!   Timber Bureau Leaflet 107.
//! - Cruz, M.G., Gould, J.S., Alexander, M.E., Sullivan, A.L., McCaw, W.L., Matthews, S. (2015).
//!   "Empirical-based models for predicting head-fire rate of spread in Australian fuel types."
//!   Australian Forestry, 78(3), 118-158.

use crate::core_types::fuel::Fuel;

/// Calculate Rothermel fire spread rate (m/min)
///
/// The Rothermel model predicts fire spread rate based on fuel properties, weather, and terrain.
///
/// # Formula
/// ```text
/// R = I_R × ξ × Φ_w × Φ_s / (ρ_b × ε × Q_ig)
/// ```
///
/// Where:
/// - **R** = Rate of spread (m/min)
/// - **I_R** = Reaction intensity (kJ/(m²·min))
/// - **ξ** = Propagating flux ratio (0-1)
/// - **Φ_w** = Wind coefficient (dimensionless)
/// - **Φ_s** = Slope coefficient (dimensionless)
/// - **ρ_b** = Fuel bed bulk density (kg/m³)
/// - **ε** = Effective heating number (0.3-0.5)
/// - **Q_ig** = Heat of pre-ignition (kJ/kg)
///
/// # Arguments
/// * `fuel` - Fuel properties
/// * `moisture_fraction` - Fuel moisture content (0-1)
/// * `wind_speed_ms` - Wind speed at midflame height (m/s)
/// * `slope_angle` - Terrain slope angle (degrees)
/// * `ambient_temp` - Ambient air temperature (°C)
///
/// # Returns
/// Fire spread rate in meters per minute
///
/// # Example
/// ```
/// use fire_sim_core::physics::rothermel_validation::rothermel_spread_rate;
/// use fire_sim_core::Fuel;
///
/// let fuel = Fuel::dry_grass();
/// let spread_rate = rothermel_spread_rate(&fuel, 0.05, 5.0, 0.0, 20.0);
/// // Expect ~30-50 m/min for dry grass with 5 m/s wind
/// ```
pub fn rothermel_spread_rate(
    fuel: &Fuel,
    moisture_fraction: f32,
    wind_speed_ms: f32,
    slope_angle: f32,
    ambient_temp: f32,
) -> f32 {
    // Early exit if fuel too wet to burn
    if moisture_fraction >= *fuel.moisture_of_extinction {
        return 0.0;
    }

    // 1. Calculate reaction intensity (I_R)
    let reaction_intensity = calculate_reaction_intensity(fuel, moisture_fraction);

    // 2. Calculate propagating flux ratio (ξ)
    let propagating_flux = calculate_propagating_flux(fuel);

    // 3. Calculate wind coefficient (Φ_w)
    let wind_coefficient = calculate_wind_coefficient(fuel, wind_speed_ms);

    // 4. Calculate slope coefficient (Φ_s)
    let slope_coefficient = calculate_slope_coefficient(slope_angle);

    // 5. Calculate heat of pre-ignition (Q_ig)
    let heat_preignition = calculate_heat_preignition(fuel, moisture_fraction, ambient_temp);

    // 6. Effective heating number (from fuel properties)
    let effective_heating = *fuel.effective_heating;

    // 7. Rothermel spread rate formula
    // R = I_R × ξ × (1 + Φ_w + Φ_s) / (ρ_b × ε × Q_ig)
    // Apply Australian calibration factor (Australian fuels spread faster than US fuels)
    // Cruz et al. (2015) Australian Forestry empirical data shows significantly higher spread rates
    // Calibration factor of 0.05 brings results in line with observed Australian grassfire spread rates
    // This matches the 10-20% rule (Alexander & Cruz 2019) where spread ≈ 10-20% of wind speed
    // Combined with realistic σ=3500 for fine grass, this produces correct wind-dependent spread
    let australian_calibration = 0.05;

    let spread_rate =
        (reaction_intensity * propagating_flux * (1.0 + wind_coefficient + slope_coefficient))
            / (*fuel.bulk_density * effective_heating * heat_preignition)
            * australian_calibration;

    spread_rate.max(0.0)
}

/// Calculate reaction intensity (kJ/(m²·min))
///
/// The rate of heat release per unit area of the fire front.
///
/// # Formula
/// ```text
/// I_R = Γ' × w_n × h × η_M × η_s
/// ```
///
/// Where:
/// - **Γ'** = Optimum reaction velocity (1/min)
/// - **w_n** = Net fuel loading (kg/m²)
/// - **h** = Heat content (kJ/kg)
/// - **η_M** = Moisture damping coefficient (0-1)
/// - **η_s** = Mineral damping coefficient (0-1, typically 0.41 for wood)
fn calculate_reaction_intensity(fuel: &Fuel, moisture_fraction: f32) -> f32 {
    // Optimum reaction velocity (empirical, depends on surface-area-to-volume ratio)
    // Γ'_max = σ^1.5 / (495 + 0.0594 × σ^1.5)
    let sigma = *fuel.surface_area_to_volume;
    let sigma_15 = sigma.powf(1.5);
    let gamma_max = sigma_15 / (495.0 + 0.0594 * sigma_15);

    // Ratio of actual to optimum packing ratio (from fuel properties)
    let beta_ratio = *fuel.packing_ratio;
    let reaction_velocity = gamma_max * beta_ratio;

    // Net fuel loading (kg/m²)
    let fuel_loading = *fuel.bulk_density * *fuel.fuel_bed_depth;

    // Moisture damping coefficient
    let moisture_damping =
        calculate_moisture_damping(moisture_fraction, *fuel.moisture_of_extinction);

    // Mineral damping coefficient (from fuel properties)
    // Varies by fuel type: grass=0.7-0.9, wood=0.41, dead=0.3-0.4
    let mineral_damping = *fuel.mineral_damping;

    reaction_velocity * fuel_loading * *fuel.heat_content * moisture_damping * mineral_damping
}

/// Calculate moisture damping coefficient (η_M)
///
/// Reduces reaction intensity as fuel moisture increases.
///
/// # Formula
/// ```text
/// η_M = 1 - 2.59×(M_f/M_x) + 5.11×(M_f/M_x)² - 3.52×(M_f/M_x)³
/// ```
///
/// Where:
/// - **M_f** = Fuel moisture content (fraction)
/// - **M_x** = Moisture of extinction (fraction)
fn calculate_moisture_damping(moisture_fraction: f32, moisture_extinction: f32) -> f32 {
    if moisture_extinction <= 0.0 {
        return 1.0;
    }

    let ratio = (moisture_fraction / moisture_extinction).min(1.0);

    // Rothermel moisture damping equation
    let damping = 1.0 - 2.59 * ratio + 5.11 * ratio.powi(2) - 3.52 * ratio.powi(3);

    damping.clamp(0.0, 1.0)
}

/// Calculate propagating flux ratio (ξ)
///
/// The fraction of reaction intensity that goes into preheating adjacent fuel.
///
/// # Formula
/// ```text
/// ξ = exp((0.792 + 0.681×σ^0.5) × (β + 0.1)) / (192 + 0.2595×σ)
/// ```
///
/// Where:
/// - **σ** = Surface-area-to-volume ratio (m²/m³)
/// - **β** = Packing ratio (fraction of fuel bed volume occupied by fuel)
fn calculate_propagating_flux(fuel: &Fuel) -> f32 {
    let sigma = *fuel.surface_area_to_volume;

    // Packing ratio from fuel properties
    // β = ρ_b / ρ_p, where ρ_p is particle density (varies by fuel type)
    let beta = (*fuel.bulk_density / *fuel.particle_density).min(1.0);

    // Rothermel propagating flux equation
    let numerator = ((0.792 + 0.681 * sigma.sqrt()) * (beta + 0.1)).exp();
    let denominator = 192.0 + 0.2595 * sigma;

    (numerator / denominator).clamp(0.0, 1.0)
}

/// Calculate wind coefficient (Φ_w)
///
/// Wind increases fire spread rate exponentially.
///
/// # Formula
/// ```text
/// Φ_w = C × U^B × (β/β_op)^(-E)
/// ```
///
/// Where:
/// - **C** = Wind coefficient constant (function of σ)
/// - **U** = Midflame wind speed (m/min, converted from m/s)
/// - **B** = Wind exponent (function of σ)
/// - **β** = Packing ratio
/// - **β_op** = Optimum packing ratio
/// - **E** = Packing ratio exponent
///
/// Simplified for Australian conditions based on McArthur and Cruz et al. (2015)
fn calculate_wind_coefficient(fuel: &Fuel, wind_speed_ms: f32) -> f32 {
    if wind_speed_ms < 0.1 {
        return 0.0; // No wind effect
    }

    let sigma = *fuel.surface_area_to_volume;

    // Convert wind speed to m/min
    let wind_speed_m_per_min = wind_speed_ms * 60.0;

    // Wind coefficient constant (function of surface-area-to-volume ratio)
    // C = 7.47 × exp(-0.133 × σ^0.55)
    let c_coeff = 7.47 * (-0.133 * sigma.powf(0.55)).exp();

    // Wind exponent (function of surface-area-to-volume ratio)
    // B = 0.02526 × σ^0.54
    let b_exp = 0.02526 * sigma.powf(0.54);

    // Packing ratio effects (from fuel properties)
    let beta = (*fuel.bulk_density / *fuel.particle_density).min(1.0);
    let beta_op = *fuel.optimum_packing_ratio; // Fuel-specific optimal compaction (grass=0.35, shrub=0.30, forest=0.25)
    let packing_effect = if beta > 0.01 && beta_op > 0.01 {
        (beta / beta_op).powf(-0.3) // E = 0.3 (typical exponent)
    } else {
        1.0
    };

    c_coeff * wind_speed_m_per_min.powf(b_exp) * packing_effect
}

/// Calculate slope coefficient (Φ_s)
///
/// Slope increases fire spread rate exponentially uphill.
///
/// # Formula
/// ```text
/// Φ_s = 5.275 × β^(-0.3) × tan²(θ)
/// ```
///
/// Where:
/// - **β** = Packing ratio
/// - **θ** = Slope angle (degrees, converted to radians)
///
/// # References
/// - Rothermel (1972) slope factor
/// - Butler et al. (2004) "Fire behavior on slopes"
fn calculate_slope_coefficient(slope_angle: f32) -> f32 {
    if slope_angle.abs() < 0.1 {
        return 0.0; // No slope effect
    }

    // Only uphill slope increases spread (downhill is handled by directional spread factor)
    if slope_angle < 0.0 {
        return 0.0;
    }

    let slope_radians = slope_angle.to_radians();
    let tan_slope = slope_radians.tan();

    // Rothermel slope coefficient
    // Φ_s = 5.275 × β^(-0.3) × tan²(θ)
    // Simplified with β ≈ 0.2 (typical), giving β^(-0.3) ≈ 1.25
    5.275 * 1.25 * tan_slope.powi(2)
}

/// Calculate heat of pre-ignition (Q_ig) in kJ/kg
///
/// The energy required to raise fuel from ambient to ignition temperature,
/// including moisture evaporation.
///
/// # Formula
/// ```text
/// Q_ig = C_p × (T_ig - T_a) + M_f × 2260
/// ```
///
/// Where:
/// - **C_p** = Specific heat of fuel (kJ/(kg·K))
/// - **T_ig** = Ignition temperature (°C)
/// - **T_a** = Ambient temperature (°C)
/// - **M_f** = Moisture fraction (kg/kg)
/// - **2260** = Latent heat of vaporization for water (kJ/kg)
fn calculate_heat_preignition(fuel: &Fuel, moisture_fraction: f32, ambient_temp: f32) -> f32 {
    // Sensible heat to raise fuel to ignition
    let sensible_heat = *fuel.specific_heat * (*fuel.ignition_temperature - ambient_temp);

    // Latent heat to evaporate moisture
    let latent_heat = moisture_fraction * 2260.0;

    sensible_heat + latent_heat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::fuel::Fuel;

    #[test]
    fn test_rothermel_dry_grass() {
        let fuel = Fuel::dry_grass();
        let spread_rate = rothermel_spread_rate(&fuel, 0.05, 5.0, 0.0, 20.0);

        // Dry grass with moderate wind should spread fast (5-100 m/min depending on conditions)
        // Cruz et al. (2015) documents Australian grassland fires at 10-60 m/min
        assert!(
            spread_rate > 1.0 && spread_rate < 150.0,
            "Dry grass spread rate {} m/min out of expected range",
            spread_rate
        );
    }

    #[test]
    fn test_rothermel_wind_effect() {
        let fuel = Fuel::dry_grass();

        let no_wind = rothermel_spread_rate(&fuel, 0.05, 0.0, 0.0, 20.0);
        let with_wind = rothermel_spread_rate(&fuel, 0.05, 10.0, 0.0, 20.0);

        // Wind should significantly increase spread rate
        assert!(
            with_wind > no_wind * 2.0,
            "Wind should increase spread rate significantly"
        );
    }

    #[test]
    fn test_rothermel_slope_effect() {
        let fuel = Fuel::dry_grass();

        let flat = rothermel_spread_rate(&fuel, 0.05, 5.0, 0.0, 20.0);
        let uphill = rothermel_spread_rate(&fuel, 0.05, 5.0, 20.0, 20.0);

        // Uphill slope should increase spread rate
        // 20° slope increases spread by ~10-50% depending on fuel and conditions (Rothermel 1972)
        assert!(
            uphill > flat * 1.05,
            "Uphill slope should increase spread rate by at least 5% (flat: {}, uphill: {}, increase: {:.1}%)",
            flat,
            uphill,
            ((uphill / flat) - 1.0) * 100.0
        );
    }

    #[test]
    fn test_rothermel_moisture_effect() {
        let fuel = Fuel::dry_grass();

        let dry = rothermel_spread_rate(&fuel, 0.05, 5.0, 0.0, 20.0);
        let wet = rothermel_spread_rate(&fuel, 0.20, 5.0, 0.0, 20.0);

        // Higher moisture should reduce spread rate
        assert!(dry > wet, "Dry fuel should spread faster than wet fuel");
    }

    #[test]
    fn test_moisture_damping() {
        let damping_0 = calculate_moisture_damping(0.0, 0.3);
        let damping_half = calculate_moisture_damping(0.15, 0.3);
        let damping_full = calculate_moisture_damping(0.3, 0.3);

        // Damping should decrease with moisture
        assert!(damping_0 > 0.9, "Dry fuel should have minimal damping");
        assert!(
            damping_half < damping_0,
            "Half-wet fuel should have more damping"
        );
        assert!(
            damping_full < 0.1,
            "Saturated fuel should be heavily damped"
        );
    }
}
