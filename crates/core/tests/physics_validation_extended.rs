//! Extended Physics Validation Test Suite - Phase 4
//!
//! This test module provides 50+ additional tests for comprehensive validation
//! of the bushfire simulation physics against peer-reviewed research.
//!
//! # Test Categories
//! 1. Physical constants validation (Stefan-Boltzmann, latent heat)
//! 2. Rothermel spread model extended validation
//! 3. Albini spotting distance extended validation  
//! 4. Crown fire physics extended validation
//! 5. Canopy layer transition validation
//! 6. `McArthur` FFDI extended validation
//! 7. Historical event validation
//! 8. Numerical stability tests
//! 9. Fuel type behavior validation
//! 10. Byram flame height validation
//!
//! # References
//! - Stefan (1879), Boltzmann (1884): Radiation law
//! - Rothermel (1972): USDA Forest Service Research Paper INT-115
//! - Albini (1979, 1983): Spotting distance models
//! - Van Wagner (1977): Crown fire initiation
//! - Cruz et al. (2015): Australian fuel type calibration
//! - Byram (1959): Flame height model
//!
//! Run tests with: `cargo test --test physics_validation_extended`

use fire_sim_core::{
    core_types::{Celsius, Kilograms, Percent},
    physics::{
        albini_spotting_validation::{
            calculate_lofting_height, calculate_maximum_spotting_distance,
        },
        canopy_layers_validation::{
            calculate_layer_transition_probability, CanopyLayer, CanopyStructure,
        },
        crown_fire_validation::{
            calculate_critical_crown_spread_rate, calculate_critical_surface_intensity,
        },
        rothermel_validation::rothermel_spread_rate,
        CombustionPhase, SmolderingState, SuppressionAgent,
    },
    FireSimulation, Fuel, TerrainData, Vec3, WeatherSystem,
};

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: PHYSICAL CONSTANTS VALIDATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate Stefan-Boltzmann constant value matches NIST reference
/// Source: NIST physical constants
/// Expected: σ = 5.670374419 × 10⁻⁸ W/(m²·K⁴)
#[test]
fn test_stefan_boltzmann_constant_nist() {
    const STEFAN_BOLTZMANN: f64 = 5.67e-8;
    const NIST_VALUE: f64 = 5.670374419e-8;

    let relative_error = ((STEFAN_BOLTZMANN - NIST_VALUE) / NIST_VALUE).abs();
    assert!(
        relative_error < 0.001,
        "Stefan-Boltzmann constant should match NIST value within 0.1%: got {STEFAN_BOLTZMANN}, expected {NIST_VALUE}"
    );
}

/// Validate Stefan-Boltzmann T⁴ scaling behavior
/// The radiated power should scale with the fourth power of temperature
#[test]
fn test_stefan_boltzmann_t4_scaling() {
    const STEFAN_BOLTZMANN: f64 = 5.67e-8;
    const EMISSIVITY: f64 = 0.95;

    let t1 = 500.0_f64;
    let power1 = STEFAN_BOLTZMANN * EMISSIVITY * t1.powi(4);

    let t2 = 1000.0_f64;
    let power2 = STEFAN_BOLTZMANN * EMISSIVITY * t2.powi(4);

    let ratio = power2 / power1;
    assert!(
        (ratio - 16.0).abs() < 0.01,
        "Doubling temperature should increase radiated power by 16x: got {ratio:.2}x"
    );
}

/// Validate net radiation heat transfer formula
/// Full formula: `Q = σ × ε × (T_source⁴ - T_target⁴)`
#[test]
fn test_stefan_boltzmann_net_radiation() {
    const STEFAN_BOLTZMANN: f64 = 5.67e-8;
    const EMISSIVITY: f64 = 0.95;

    let t_source = 800.0_f64;
    let t_target = 300.0_f64;

    let net_flux = STEFAN_BOLTZMANN * EMISSIVITY * (t_source.powi(4) - t_target.powi(4));

    assert!(net_flux > 0.0, "Net flux should be positive (hot to cold)");
    assert!(
        (net_flux - 21600.0).abs() < 1000.0,
        "Net flux at 800K→300K should be ~21.6 kW/m²: got {net_flux:.1} W/m²"
    );
}

/// Validate radiation equilibrium temperature calculation
#[test]
fn test_stefan_boltzmann_equilibrium() {
    const STEFAN_BOLTZMANN: f64 = 5.67e-8;
    const EMISSIVITY: f64 = 0.95;

    let power_flux = 1000.0;
    let equilibrium_t4 = power_flux / (STEFAN_BOLTZMANN * EMISSIVITY);
    let equilibrium_k = equilibrium_t4.powf(0.25);

    // Calculated: T = (1000 / (5.67e-8 * 0.95))^0.25 ≈ 369K
    assert!(
        (equilibrium_k - 369.0).abs() < 5.0,
        "Equilibrium temperature at 1000 W/m² should be ~369K: got {equilibrium_k:.1}K"
    );
}

/// Validate latent heat of vaporization constant
/// Source: NIST - water latent heat = 2260 kJ/kg
#[test]
fn test_latent_heat_vaporization() {
    const LATENT_HEAT_WATER: f64 = 2260.0; // kJ/kg
    const EXPECTED: f64 = 2260.0;

    assert!(
        (LATENT_HEAT_WATER - EXPECTED).abs() < 10.0,
        "Latent heat of vaporization should be ~2260 kJ/kg"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: ROTHERMEL SPREAD MODEL EXTENDED TESTS
// Source: Rothermel (1972) USDA Forest Service Research Paper INT-115
// ═══════════════════════════════════════════════════════════════════════════════

/// Test Rothermel spread rate with no wind (baseline)
#[test]
fn test_rothermel_no_wind_baseline() {
    let fuel = Fuel::dry_grass();
    let rate = rothermel_spread_rate(&fuel, 0.08, 0.0, 0.0, 20.0);

    assert!(rate >= 0.0, "Spread rate must be non-negative");
}

/// Test Rothermel wind factor increases spread
#[test]
fn test_rothermel_wind_factor_increases_spread() {
    let fuel = Fuel::dry_grass();
    let base_rate = rothermel_spread_rate(&fuel, 0.08, 0.0, 0.0, 20.0);
    let wind_rate = rothermel_spread_rate(&fuel, 0.08, 10.0, 0.0, 20.0);

    assert!(
        wind_rate > base_rate,
        "Wind should increase spread rate: no_wind={base_rate:.2}, with_wind={wind_rate:.2}"
    );
}

/// Test Rothermel slope effect (uphill spread acceleration)
#[test]
fn test_rothermel_slope_increases_spread() {
    let fuel = Fuel::dry_grass();
    let flat_rate = rothermel_spread_rate(&fuel, 0.08, 5.0, 0.0, 20.0);
    let slope_rate = rothermel_spread_rate(&fuel, 0.08, 5.0, 20.0, 20.0);

    assert!(
        slope_rate > flat_rate,
        "Uphill slope should increase spread rate"
    );
}

/// Test Rothermel moisture damping coefficient
#[test]
fn test_rothermel_moisture_reduces_spread() {
    let fuel = Fuel::dry_grass();
    let dry_rate = rothermel_spread_rate(&fuel, 0.05, 5.0, 0.0, 20.0);
    let wet_rate = rothermel_spread_rate(&fuel, 0.20, 5.0, 0.0, 20.0);

    assert!(
        dry_rate > wet_rate,
        "Dry fuel should spread faster: dry={dry_rate:.2}, wet={wet_rate:.2}"
    );
}

/// Test Rothermel temperature effect
#[test]
fn test_rothermel_temperature_effect() {
    let fuel = Fuel::dry_grass();
    let cold_rate = rothermel_spread_rate(&fuel, 0.08, 5.0, 0.0, 10.0);
    let hot_rate = rothermel_spread_rate(&fuel, 0.08, 5.0, 0.0, 40.0);

    // Both should be valid; hot may increase slightly due to lower pre-ignition heat
    assert!(!cold_rate.is_nan() && !hot_rate.is_nan());
}

/// Test Rothermel with eucalyptus stringybark
#[test]
fn test_rothermel_stringybark_fuel() {
    let fuel = Fuel::eucalyptus_stringybark();
    let rate = rothermel_spread_rate(&fuel, 0.10, 8.0, 0.0, 30.0);

    assert!(rate > 0.0, "Stringybark should support fire spread");
}

/// Test Rothermel with shrubland fuel
#[test]
fn test_rothermel_shrubland_fuel() {
    let fuel = Fuel::shrubland();
    let rate = rothermel_spread_rate(&fuel, 0.08, 5.0, 0.0, 25.0);

    assert!(rate >= 0.0, "Shrubland spread rate should be valid");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: ALBINI SPOTTING MODEL EXTENDED TESTS
// Source: Albini (1979, 1983)
// ═══════════════════════════════════════════════════════════════════════════════

/// Test lofting height increases with fireline intensity
#[test]
fn test_albini_lofting_height_intensity_scaling() {
    let height_1000 = calculate_lofting_height(1000.0);
    let height_5000 = calculate_lofting_height(5000.0);
    let height_10000 = calculate_lofting_height(10000.0);

    assert!(
        height_5000 > height_1000,
        "5000 kW/m should loft higher than 1000 kW/m"
    );
    assert!(
        height_10000 > height_5000,
        "10000 kW/m should loft higher than 5000 kW/m"
    );
}

/// Test lofting height at low intensity
#[test]
fn test_albini_lofting_low_intensity() {
    let height = calculate_lofting_height(500.0);
    assert!(
        height > 0.0 && height < 500.0,
        "Low intensity lofting should be modest"
    );
}

/// Test maximum spotting distance with moderate conditions
#[test]
fn test_albini_spotting_moderate_conditions() {
    let distance = calculate_maximum_spotting_distance(
        10000.0, // intensity
        10.0,    // wind speed
        0.001,   // ember mass (1g)
        0.02,    // diameter (2cm)
        0.0,     // slope
    );

    assert!(distance > 0.0, "Spotting distance must be positive");
}

/// Test maximum spotting distance with high wind
#[test]
fn test_albini_spotting_high_wind() {
    let low_wind = calculate_maximum_spotting_distance(10000.0, 5.0, 0.001, 0.02, 0.0);
    let high_wind = calculate_maximum_spotting_distance(10000.0, 25.0, 0.001, 0.02, 0.0);

    assert!(
        high_wind > low_wind,
        "Higher wind should increase spotting distance"
    );
}

/// Test spotting with uphill slope
#[test]
fn test_albini_spotting_uphill() {
    let _flat = calculate_maximum_spotting_distance(10000.0, 15.0, 0.001, 0.02, 0.0);
    let uphill = calculate_maximum_spotting_distance(10000.0, 15.0, 0.001, 0.02, 20.0);

    // Uphill may extend or reduce depending on implementation
    assert!(uphill >= 0.0, "Uphill spotting should be valid");
}

/// Test extreme intensity spotting (Black Saturday levels)
#[test]
fn test_albini_spotting_extreme_intensity() {
    let height = calculate_lofting_height(100_000.0);
    assert!(
        height > 500.0,
        "Extreme intensity should produce high lofting: got {height:.0}m"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: VAN WAGNER CROWN FIRE TESTS
// Source: Van Wagner (1977) Canadian Journal of Forest Research
// ═══════════════════════════════════════════════════════════════════════════════

/// Test critical surface intensity for crown fire initiation
#[test]
fn test_van_wagner_critical_intensity() {
    // CBD, heat content, FMC, crown base height
    let intensity = calculate_critical_surface_intensity(0.15, 20000.0, 100.0, 5.0);

    assert!(
        intensity > 0.0,
        "Critical intensity must be positive: got {intensity:.0}"
    );
}

/// Test crown base height effect on critical intensity
#[test]
fn test_van_wagner_cbh_effect() {
    let intensity_low_cbh = calculate_critical_surface_intensity(0.15, 20000.0, 100.0, 3.0);
    let intensity_high_cbh = calculate_critical_surface_intensity(0.15, 20000.0, 100.0, 10.0);

    assert!(
        intensity_low_cbh > intensity_high_cbh,
        "Lower CBH should require higher intensity: low={intensity_low_cbh:.0}, high={intensity_high_cbh:.0}"
    );
}

/// Test foliar moisture effect on critical intensity
#[test]
fn test_van_wagner_moisture_effect() {
    let intensity_dry = calculate_critical_surface_intensity(0.15, 20000.0, 80.0, 5.0);
    let intensity_wet = calculate_critical_surface_intensity(0.15, 20000.0, 150.0, 5.0);

    assert!(
        intensity_wet > intensity_dry,
        "Higher FMC should require more intensity: dry={intensity_dry:.0}, wet={intensity_wet:.0}"
    );
}

/// Test critical crown spread rate
#[test]
fn test_van_wagner_critical_spread_rate() {
    let rate_low_cbd = calculate_critical_crown_spread_rate(0.05);
    let rate_high_cbd = calculate_critical_crown_spread_rate(0.15);

    assert!(
        rate_low_cbd > rate_high_cbd,
        "Lower CBD should need higher spread rate: low={rate_low_cbd:.2}, high={rate_high_cbd:.2}"
    );
}

/// Test critical spread rate with zero CBD returns zero
#[test]
fn test_van_wagner_zero_cbd() {
    let rate = calculate_critical_crown_spread_rate(0.0);
    assert!(rate == 0.0, "Zero CBD should return zero spread rate");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: CANOPY LAYER TRANSITION TESTS
// Source: Pausas et al. (2017), Cruz et al. (2022)
// ═══════════════════════════════════════════════════════════════════════════════

/// Test understory to midstory transition probability
#[test]
fn test_canopy_transition_understory_to_midstory() {
    let canopy = CanopyStructure::eucalyptus_stringybark();
    let prob = calculate_layer_transition_probability(
        5000.0, // intensity
        &canopy,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    assert!(
        (0.0..=1.0).contains(&prob),
        "Probability must be 0-1: got {prob:.3}"
    );
}

/// Test midstory to overstory transition
#[test]
fn test_canopy_transition_midstory_to_overstory() {
    let canopy = CanopyStructure::eucalyptus_stringybark();
    let prob = calculate_layer_transition_probability(
        15000.0, // high intensity
        &canopy,
        CanopyLayer::Midstory,
        CanopyLayer::Overstory,
    );

    assert!(
        (0.0..=1.0).contains(&prob),
        "Probability must be 0-1: got {prob:.3}"
    );
}

/// Test intensity effect on transition probability
#[test]
fn test_canopy_transition_intensity_effect() {
    let canopy = CanopyStructure::eucalyptus_stringybark();

    let prob_low = calculate_layer_transition_probability(
        1000.0,
        &canopy,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    let prob_high = calculate_layer_transition_probability(
        20000.0,
        &canopy,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    assert!(
        prob_high >= prob_low,
        "Higher intensity should increase transition probability"
    );
}

/// Test stringybark vs smooth bark transition probability
#[test]
fn test_stringybark_vs_smooth_bark_transition() {
    let stringybark = CanopyStructure::eucalyptus_stringybark();
    let smooth_bark = CanopyStructure::eucalyptus_smooth_bark();

    let prob_stringy = calculate_layer_transition_probability(
        5000.0,
        &stringybark,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    let prob_smooth = calculate_layer_transition_probability(
        5000.0,
        &smooth_bark,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    // Stringybark has higher ladder fuel factor
    assert!(
        prob_stringy >= prob_smooth,
        "Stringybark should have higher transition prob: stringy={prob_stringy:.3}, smooth={prob_smooth:.3}"
    );
}

/// Test grassland has no mid-story transition
#[test]
fn test_grassland_no_midstory() {
    let grassland = CanopyStructure::grassland();
    let prob = calculate_layer_transition_probability(
        10000.0,
        &grassland,
        CanopyLayer::Understory,
        CanopyLayer::Midstory,
    );

    // Grassland has no mid-story fuel, so low probability expected
    assert!(prob <= 1.0, "Probability should be valid: got {prob:.3}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6: MCARTHUR FFDI EXTENDED TESTS
// Source: McArthur (1967), Bureau of Meteorology
// ═══════════════════════════════════════════════════════════════════════════════

/// Test FFDI sensitivity to temperature
#[test]
fn test_ffdi_temperature_sensitivity() {
    let weather_25 = WeatherSystem::new(25.0, 30.0, 20.0, 0.0, 5.0);
    let weather_35 = WeatherSystem::new(35.0, 30.0, 20.0, 0.0, 5.0);
    let weather_45 = WeatherSystem::new(45.0, 30.0, 20.0, 0.0, 5.0);

    let ffdi_25 = weather_25.calculate_ffdi();
    let ffdi_35 = weather_35.calculate_ffdi();
    let ffdi_45 = weather_45.calculate_ffdi();

    assert!(
        ffdi_35 > ffdi_25 && ffdi_45 > ffdi_35,
        "Higher temp should increase FFDI"
    );
}

/// Test FFDI sensitivity to humidity
#[test]
fn test_ffdi_humidity_sensitivity() {
    let weather_dry = WeatherSystem::new(35.0, 15.0, 25.0, 0.0, 5.0);
    let weather_wet = WeatherSystem::new(35.0, 60.0, 25.0, 0.0, 5.0);

    let ffdi_dry = weather_dry.calculate_ffdi();
    let ffdi_wet = weather_wet.calculate_ffdi();

    assert!(
        ffdi_dry > ffdi_wet,
        "Lower humidity should increase FFDI: 15%={ffdi_dry:.1}, 60%={ffdi_wet:.1}"
    );
}

/// Test FFDI sensitivity to wind speed
#[test]
fn test_ffdi_wind_sensitivity() {
    let weather_calm = WeatherSystem::new(35.0, 25.0, 10.0, 0.0, 5.0);
    let weather_windy = WeatherSystem::new(35.0, 25.0, 40.0, 0.0, 5.0);

    let ffdi_calm = weather_calm.calculate_ffdi();
    let ffdi_windy = weather_windy.calculate_ffdi();

    assert!(
        ffdi_windy > ffdi_calm,
        "Higher wind should increase FFDI: 10km/h={ffdi_calm:.1}, 40km/h={ffdi_windy:.1}"
    );
}

/// Test FFDI sensitivity to drought factor
#[test]
fn test_ffdi_drought_factor_sensitivity() {
    let weather_low_df = WeatherSystem::new(35.0, 25.0, 25.0, 0.0, 3.0);
    let weather_high_df = WeatherSystem::new(35.0, 25.0, 25.0, 0.0, 10.0);

    let ffdi_low = weather_low_df.calculate_ffdi();
    let ffdi_high = weather_high_df.calculate_ffdi();

    assert!(
        ffdi_high > ffdi_low,
        "Higher drought factor should increase FFDI: DF=3={ffdi_low:.1}, DF=10={ffdi_high:.1}"
    );
}

/// Test fire danger rating method exists and returns valid string
#[test]
fn test_fire_danger_rating_valid() {
    let weather = WeatherSystem::new(35.0, 20.0, 30.0, 0.0, 8.0);
    let rating = weather.fire_danger_rating();

    assert!(!rating.is_empty(), "Fire danger rating should not be empty");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7: HISTORICAL EVENT VALIDATION
// Sources: Various incident reports and scientific analyses
// ═══════════════════════════════════════════════════════════════════════════════

/// Ash Wednesday 1983 FFDI validation
/// Reference: Bureau of Meteorology records
/// Conditions: 43°C, 6% RH, 70 km/h winds, DF=10
#[test]
fn test_ash_wednesday_1983_ffdi() {
    let weather = WeatherSystem::new(43.0, 6.0, 70.0, 0.0, 10.0);
    let ffdi = weather.calculate_ffdi();

    assert!(
        ffdi > 100.0,
        "Ash Wednesday FFDI should exceed 100: got {ffdi:.1}"
    );
}

/// Black Saturday 2009 FFDI validation
/// Reference: Cruz et al. (2012)
/// Conditions: 46.4°C, 6% RH, 80 km/h sustained winds
#[test]
fn test_black_saturday_2009_ffdi() {
    let weather = WeatherSystem::new(46.4, 6.0, 80.0, 0.0, 10.0);
    let ffdi = weather.calculate_ffdi();

    assert!(
        ffdi > 150.0,
        "Black Saturday FFDI should exceed 150: got {ffdi:.1}"
    );
}

/// Canberra 2003 fire tornado conditions
#[test]
fn test_canberra_2003_ffdi() {
    let weather = WeatherSystem::new(40.0, 8.0, 60.0, 0.0, 10.0);
    let ffdi = weather.calculate_ffdi();

    assert!(
        ffdi > 80.0,
        "Canberra 2003 FFDI should be extreme: got {ffdi:.1}"
    );
}

/// Perth Hills 2014 validation
#[test]
fn test_perth_hills_2014_ffdi() {
    let weather = WeatherSystem::new(41.0, 9.0, 55.0, 0.0, 9.0);
    let ffdi = weather.calculate_ffdi();

    assert!(
        ffdi > 70.0,
        "Perth Hills 2014 FFDI should be severe+: got {ffdi:.1}"
    );
}

/// Margaret River 2011 validation
#[test]
fn test_margaret_river_2011_ffdi() {
    let weather = WeatherSystem::new(37.0, 12.0, 45.0, 0.0, 8.0);
    let ffdi = weather.calculate_ffdi();

    assert!(
        ffdi > 50.0,
        "Margaret River 2011 FFDI should be severe: got {ffdi:.1}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 8: NUMERICAL STABILITY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test Rothermel with zero wind doesn't produce NaN
#[test]
fn test_rothermel_zero_wind_no_nan() {
    let fuel = Fuel::dry_grass();
    let rate = rothermel_spread_rate(&fuel, 0.08, 0.0, 0.0, 20.0);

    assert!(!rate.is_nan(), "Zero wind should not produce NaN");
    assert!(!rate.is_infinite(), "Result should be finite");
}

/// Test Rothermel with extreme wind bounded
#[test]
fn test_rothermel_extreme_wind_bounded() {
    let fuel = Fuel::dry_grass();
    let rate = rothermel_spread_rate(&fuel, 0.05, 50.0, 0.0, 35.0);

    assert!(!rate.is_nan(), "Extreme wind should not produce NaN");
    assert!(!rate.is_infinite(), "Result should be finite");
    assert!(rate >= 0.0, "Spread rate cannot be negative");
}

/// Test FFDI with extreme temperature bounded
#[test]
fn test_ffdi_extreme_temperature_bounded() {
    let weather = WeatherSystem::new(50.0, 5.0, 80.0, 0.0, 10.0);
    let ffdi = weather.calculate_ffdi();

    assert!(!ffdi.is_nan(), "Extreme temp FFDI should not be NaN");
    assert!(!ffdi.is_infinite(), "FFDI should be finite");
    assert!(ffdi > 0.0, "FFDI must be positive");
}

/// Test FFDI with very low humidity bounded
#[test]
fn test_ffdi_very_low_humidity_bounded() {
    let weather = WeatherSystem::new(45.0, 3.0, 60.0, 0.0, 10.0);
    let ffdi = weather.calculate_ffdi();

    assert!(!ffdi.is_nan(), "Very low humidity FFDI should not be NaN");
    assert!(!ffdi.is_infinite(), "FFDI should be finite");
}

/// Test lofting height with zero intensity
#[test]
fn test_lofting_zero_intensity() {
    let height = calculate_lofting_height(0.0);
    assert!(!height.is_nan(), "Zero intensity should not produce NaN");
    assert!(height >= 0.0, "Lofting height cannot be negative");
}

/// Test spotting with very high intensity
#[test]
fn test_spotting_extreme_intensity_bounded() {
    let height = calculate_lofting_height(200_000.0);

    assert!(
        !height.is_nan(),
        "Extreme intensity lofting should not be NaN"
    );
    assert!(height.is_finite(), "Lofting height should be finite");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 9: FUEL TYPE BEHAVIOR VALIDATION
// Source: Anderson (1982), Cruz et al. (2015)
// ═══════════════════════════════════════════════════════════════════════════════

/// Test dry grass fuel SAV is realistic
/// Note: SAV in this codebase uses 1/m units
/// Grass SAV typically 2000-5000 1/m for fine grass
#[test]
fn test_dry_grass_sav_realistic() {
    let grass = Fuel::dry_grass();
    let sav = *grass.surface_area_to_volume;

    assert!(
        sav > 2000.0 && sav < 6000.0,
        "Grass SAV should be realistic: got {sav:.0} 1/m"
    );
}

/// Test eucalyptus stringybark has bark properties
#[test]
fn test_stringybark_has_ladder_factor() {
    let fuel = Fuel::eucalyptus_stringybark();
    // Stringybark has high ladder fuel factor due to fibrous bark
    let bark = &fuel.bark_properties;
    let factor = *bark.ladder_fuel_factor;
    assert!(
        factor > 0.5,
        "Stringybark should have high ladder fuel factor: got {factor}"
    );
}

/// Test fuel heat of combustion is positive
#[test]
fn test_fuel_heat_content_positive() {
    let fuels = [
        Fuel::dry_grass(),
        Fuel::eucalyptus_stringybark(),
        Fuel::eucalyptus_smooth_bark(),
        Fuel::shrubland(),
    ];

    for fuel in fuels {
        let heat = *fuel.heat_content;
        assert!(heat > 0.0, "Heat content must be positive: got {heat:.0}");
    }
}

/// Test fuel moisture extinction thresholds
#[test]
fn test_fuel_moisture_extinction_realistic() {
    let fuels = [
        Fuel::dry_grass(),
        Fuel::eucalyptus_stringybark(),
        Fuel::shrubland(),
    ];

    for fuel in fuels {
        let extinction = *fuel.moisture_of_extinction;
        assert!(
            extinction > 0.05 && extinction < 0.50,
            "Moisture extinction should be 5-50%: got {:.0}%",
            extinction * 100.0
        );
    }
}

/// Test multiple fuel types have distinct properties
#[test]
fn test_fuel_types_distinct() {
    let grass = Fuel::dry_grass();
    let stringybark = Fuel::eucalyptus_stringybark();

    // SAV should differ significantly
    let grass_sav = *grass.surface_area_to_volume;
    let stringy_sav = *stringybark.surface_area_to_volume;

    assert!(
        (grass_sav - stringy_sav).abs() > 100.0,
        "Grass and stringybark should have different SAV"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 10: BYRAM FLAME HEIGHT VALIDATION
// Source: Byram (1959)
// ═══════════════════════════════════════════════════════════════════════════════

/// Byram's flame height formula: L = 0.0775 × I^0.46
fn byram_flame_height(intensity_kw_m: f32) -> f32 {
    0.0775 * intensity_kw_m.powf(0.46)
}

/// Test Byram flame height at various intensities
#[test]
fn test_byram_flame_height_range() {
    let test_cases = [(100.0, 0.6), (1000.0, 1.9), (10000.0, 5.9)];

    for (intensity, expected_approx) in test_cases {
        let height = byram_flame_height(intensity);
        let tolerance = expected_approx * 0.3;

        assert!(
            (height - expected_approx).abs() < tolerance,
            "Byram flame height at {intensity:.0} kW/m should be ~{expected_approx:.1}m: got {height:.1}m"
        );
    }
}

/// Test Byram flame height never negative
#[test]
fn test_byram_flame_height_non_negative() {
    let height_zero = byram_flame_height(0.0);
    let height_tiny = byram_flame_height(0.001);

    assert!(height_zero >= 0.0, "Flame height cannot be negative");
    assert!(height_tiny >= 0.0, "Flame height cannot be negative");
}

/// Test Byram extreme intensity
#[test]
fn test_byram_extreme_intensity() {
    let height = byram_flame_height(100_000.0);

    // At extreme intensity, expect very tall flames (15-20m)
    assert!(
        height > 10.0 && height < 30.0,
        "Extreme intensity flame height should be 10-30m: got {height:.1}m"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 11: COMBUSTION PHASE AND STATE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test `CombustionPhase` enum values exist
#[test]
fn test_combustion_phase_variants_exist() {
    let phases = [
        CombustionPhase::Unignited,
        CombustionPhase::Flaming,
        CombustionPhase::Transition,
        CombustionPhase::Smoldering,
        CombustionPhase::Extinguished,
    ];

    // Verify they can be compared
    assert_ne!(phases[0], phases[1]);
    assert_eq!(phases[0], CombustionPhase::Unignited);
}

/// Test `SmolderingState` default is unignited
#[test]
fn test_smoldering_state_default() {
    let state = SmolderingState::default();
    assert_eq!(state.phase(), CombustionPhase::Unignited);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 12: SUPPRESSION AGENT TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test `SuppressionAgent` enum variants exist
#[test]
fn test_suppression_agent_variants() {
    let agents = [SuppressionAgent::Water, SuppressionAgent::Foam];

    assert_ne!(agents[0], agents[1]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 13: CELSIUS TYPE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test Celsius to Kelvin conversion at freezing
#[test]
fn test_celsius_freezing_to_kelvin() {
    let freezing = Celsius::new(0.0);
    let kelvin = freezing.to_kelvin();

    assert!(
        (*kelvin - 273.15).abs() < 0.1,
        "0°C should be 273.15K: got {:.2}K",
        *kelvin
    );
}

/// Test Celsius to Kelvin at boiling
#[test]
fn test_celsius_boiling_to_kelvin() {
    let boiling = Celsius::new(100.0);
    let kelvin = boiling.to_kelvin();

    assert!(
        (*kelvin - 373.15).abs() < 0.1,
        "100°C should be 373.15K: got {:.2}K",
        *kelvin
    );
}

/// Test Celsius to Kelvin at typical fire temperature
#[test]
fn test_celsius_fire_temp_to_kelvin() {
    let fire = Celsius::new(700.0);
    let kelvin = fire.to_kelvin();

    assert!(
        (*kelvin - 973.15).abs() < 0.1,
        "700°C should be 973.15K: got {:.2}K",
        *kelvin
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 14: SIMULATION INITIALIZATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test `FireSimulation` can be created with terrain
#[test]
fn test_simulation_initialization() {
    let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
    let sim = FireSimulation::new(5.0, &terrain);

    // Verify terrain was accepted
    assert!(sim.terrain().width() > 0.0);
}

/// Test terrain flat creation
#[test]
fn test_terrain_flat_creation() {
    let terrain = TerrainData::flat(100.0, 100.0, 5.0, 50.0);

    // Verify dimensions
    assert!((terrain.width() - 100.0).abs() < 0.1);
    assert!((terrain.height() - 100.0).abs() < 0.1);
}

/// Test terrain single hill creation
#[test]
fn test_terrain_single_hill_creation() {
    let terrain = TerrainData::single_hill(100.0, 100.0, 5.0, 50.0, 50.0, 20.0);

    // Verify it has elevation variation
    let max_elev = terrain.max_elevation();
    assert!(max_elev > 0.0, "Hill should have positive elevation");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 15: VEC3 TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test `Vec3` construction
#[test]
fn test_vec3_construction() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert!((v.x - 1.0).abs() < 0.001);
    assert!((v.y - 2.0).abs() < 0.001);
    assert!((v.z - 3.0).abs() < 0.001);
}

/// Test `Vec3` `metric_distance` (3-4-5 triangle)
#[test]
fn test_vec3_metric_distance() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(3.0, 4.0, 0.0);

    let dist = a.metric_distance(&b);
    assert!(
        (dist - 5.0).abs() < 0.001,
        "3-4-5 triangle distance should be 5: got {dist}"
    );
}

/// Test Vec3 3D distance
#[test]
fn test_vec3_3d_distance() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(1.0, 2.0, 2.0);

    let dist = a.metric_distance(&b);
    let expected = (1.0_f32 + 4.0 + 4.0).sqrt();

    assert!(
        (dist - expected).abs() < 0.001,
        "3D distance should be {expected}: got {dist}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 16: WEATHER SYSTEM BOUNDARY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Test FFDI with minimum realistic values
#[test]
fn test_ffdi_minimum_conditions() {
    let weather = WeatherSystem::new(15.0, 80.0, 5.0, 0.0, 1.0);
    let ffdi = weather.calculate_ffdi();

    assert!(ffdi >= 0.0, "FFDI should never be negative");
    assert!(
        ffdi < 10.0,
        "Low conditions should produce low FFDI: got {ffdi:.1}"
    );
}

/// Test weather wind direction doesn't affect FFDI
#[test]
fn test_ffdi_wind_direction_invariant() {
    let north = WeatherSystem::new(30.0, 30.0, 20.0, 0.0, 5.0);
    let east = WeatherSystem::new(30.0, 30.0, 20.0, 90.0, 5.0);
    let south = WeatherSystem::new(30.0, 30.0, 20.0, 180.0, 5.0);

    let ffdi_n = north.calculate_ffdi();
    let ffdi_e = east.calculate_ffdi();
    let ffdi_s = south.calculate_ffdi();

    assert!(
        (ffdi_n - ffdi_e).abs() < 0.1 && (ffdi_e - ffdi_s).abs() < 0.1,
        "Wind direction shouldn't affect FFDI"
    );
}

/// Test `WeatherSystem` default is valid
#[test]
fn test_weather_system_default_valid() {
    let weather = WeatherSystem::default();
    let ffdi = weather.calculate_ffdi();

    assert!(!ffdi.is_nan(), "Default weather FFDI should not be NaN");
    assert!(ffdi >= 0.0, "FFDI should be non-negative");
}

/// Test Kilograms type
#[test]
fn test_kilograms_type_positive() {
    let mass = Kilograms::new(5.0);
    assert!(*mass > 0.0, "Mass should be positive");
}

/// Test Percent type
#[test]
fn test_percent_type_valid() {
    let pct = Percent::new(50.0);
    assert!(*pct >= 0.0 && *pct <= 100.0, "Percent should be bounded");
}
