//! Unit test for `WeatherPreset::basic` and `WeatherSystem::from_preset`
use fire_sim_core::core_types::{Celsius, KilometersPerHour, Percent};
use fire_sim_core::{ClimatePattern, WeatherPreset, WeatherSystem};

#[test]
fn test_weather_preset_basic_and_from_preset() {
    // Create a simple custom preset
    let preset = WeatherPreset::basic(
        "Test Basic",
        Celsius::new(15.0),
        Celsius::new(30.0),
        Percent::new(25.0),
        KilometersPerHour::new(20.0),
        0.10,
    );

    // Basic fields should match inputs
    assert_eq!(preset.summer_humidity, Percent::new(25.0));
    assert_eq!(preset.summer_wind, KilometersPerHour::new(20.0));
    assert_eq!(preset.summer_drought_rate, 0.10);

    // Create a WeatherSystem from the preset
    let system = WeatherSystem::from_preset(preset.clone(), 15, 14.0, ClimatePattern::Neutral);

    // At 2pm, temperature should be equal to the provided max_temp
    let temp = system.temperature();
    assert_eq!(*temp, 30.0);

    // Humidity should match preset's summer_humidity (approx) via get_stats()
    let stats = system.get_stats();
    let hum = *stats.humidity;
    // With a midday temperature of 30°C, the preset humidity of 25% is adjusted
    // by temperature in get_humidity(): humidity -= (T - 25) * 0.5 → 25 - (30-25)*0.5 = 22.5
    assert!((hum - 22.5).abs() < 0.1);

    // Independently verify the FFDI calculation using an explicitly-configured
    // weather system (bypasses smoothing targets). This ensures FFDI behavior
    // is consistent for hot/dry/windy scenarios with high drought factor.
    let sys2 = WeatherSystem::new(38.0, 20.0, 35.0, 0.0, 8.0);
    let ffdi = sys2.calculate_ffdi();
    assert!(
        ffdi > 40.0,
        "FFDI should be high for hot/dry/windy with high drought factor: {ffdi}"
    );
}
