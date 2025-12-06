//! Nelson Fuel Moisture Timelag System (2000)
//!
//! Implements dynamic fuel moisture equilibrium with timelag classes:
//! - 1-hour timelag fuels (fine, <6mm diameter)
//! - 10-hour timelag fuels (medium, 6-25mm)
//! - 100-hour timelag fuels (coarse, 25-75mm)
//! - 1000-hour timelag fuels (very coarse, >75mm)
//!
//! # Scientific References
//! - Nelson, R.M. (2000). "Prediction of diurnal change in 10-h fuel stick moisture content"
//!   Canadian Journal of Forest Research, 30(7), 1071-1087
//! - Viney, N.R. (1991). "A review of fine fuel moisture modelling"
//!   International Journal of Wildland Fire, 1(4), 215-234
//! - Matthews, S. (2006). "A process-based model of fine fuel moisture"
//!   International Journal of Wildland Fire, 15(2), 155-168

use crate::core_types::fuel::Fuel;
use serde::{Deserialize, Serialize};

/// Calculate equilibrium moisture content based on temperature and humidity
///
/// Simard (1968) empirical equation:
/// EMC = a + b×H + c×T + d×H×T
///
/// Where coefficients depend on whether fuel is adsorbing or desorbing
///
/// # Arguments
/// * `temperature` - Air temperature (°C)
/// * `humidity` - Relative humidity (%)
/// * `is_adsorbing` - true if fuel is gaining moisture, false if losing
///
/// # Returns
/// Equilibrium moisture content (fraction 0-1)
///
/// # References
/// Simard (1968), Nelson (2000)
pub(crate) fn calculate_equilibrium_moisture(
    temperature: f32,
    humidity: f32,
    is_adsorbing: bool,
) -> f32 {
    // Simard (1968) coefficients for adsorption and desorption
    // Modified Nelson (2000) formulation for better temperature response
    let (a, b, c, d) = if is_adsorbing {
        // Adsorption (fuel gaining moisture)
        // Coefficients adjusted so temperature has negative effect
        (0.0, 0.00253, -0.000116, -0.0000158)
    } else {
        // Desorption (fuel losing moisture)
        (0.0, 0.00282, -0.000176, -0.0000201)
    };

    // Simard equation: EMC = a + b×H + c×T + d×H×T
    // With negative d, higher temperature reduces the humidity effect
    let emc = a + b * humidity + c * temperature + d * humidity * temperature;

    // Clamp to reasonable range
    emc.clamp(0.01, 0.40)
}

/// Calculate moisture timelag rate constant
///
/// The timelag constant (τ) determines how quickly fuel moisture equilibrates
///
/// dM/dt = (M_e - M) / τ
///
/// # Arguments
/// * `timelag_hours` - Characteristic timelag (hours)
///
/// # Returns
/// Rate constant for moisture equilibration (1/hours)
///
/// # References
/// Nelson (2000)
pub(crate) fn timelag_rate_constant(timelag_hours: f32) -> f32 {
    if timelag_hours <= 0.0 {
        return 0.0;
    }
    1.0 / timelag_hours
}

/// Update fuel moisture for a specific size class using timelag dynamics
///
/// Nelson (2000) exponential lag equation:
/// M(t+dt) = M_e + (M(t) - M_e) × exp(-dt / τ)
///
/// # Arguments
/// * `current_moisture` - Current moisture content (fraction 0-1)
/// * `equilibrium_moisture` - Target equilibrium moisture (fraction 0-1)
/// * `timelag_hours` - Characteristic timelag (hours)
/// * `dt_hours` - Time step (hours)
///
/// # Returns
/// New moisture content (fraction 0-1)
///
/// # References
/// Nelson (2000), Equation 3
pub(crate) fn update_moisture_timelag(
    current_moisture: f32,
    equilibrium_moisture: f32,
    timelag_hours: f32,
    dt_hours: f32,
) -> f32 {
    // Get rate constant using timelag_rate_constant
    let rate = timelag_rate_constant(timelag_hours);
    if rate <= 0.0 {
        return equilibrium_moisture;
    }

    // Nelson (2000) exponential lag equation: M(t+dt) = M_e + (M(t) - M_e) × exp(-dt × rate)
    let lag_factor = (-dt_hours * rate).exp();
    let new_moisture =
        equilibrium_moisture + (current_moisture - equilibrium_moisture) * lag_factor;

    new_moisture.clamp(0.01, 1.0)
}

/// Calculate weighted average moisture across all size classes
///
/// # Arguments
/// * `moisture_1h` - Moisture in 1-hour fuels (fraction)
/// * `moisture_10h` - Moisture in 10-hour fuels (fraction)
/// * `moisture_100h` - Moisture in 100-hour fuels (fraction)
/// * `moisture_1000h` - Moisture in 1000-hour fuels (fraction)
/// * `distribution` - Size class distribution [1h, 10h, 100h, 1000h]
///
/// # Returns
/// Weighted average moisture content (fraction 0-1)
pub(crate) fn calculate_weighted_moisture(
    moisture_1h: f32,
    moisture_10h: f32,
    moisture_100h: f32,
    moisture_1000h: f32,
    distribution: [f32; 4],
) -> f32 {
    let total = moisture_1h * distribution[0]
        + moisture_10h * distribution[1]
        + moisture_100h * distribution[2]
        + moisture_1000h * distribution[3];

    total.clamp(0.01, 1.0)
}

/// Fuel moisture state for all timelag classes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct FuelMoistureState {
    /// Moisture in 1-hour fuels (fraction 0-1)
    moisture_1h: f32,
    /// Moisture in 10-hour fuels (fraction 0-1)
    moisture_10h: f32,
    /// Moisture in 100-hour fuels (fraction 0-1)
    moisture_100h: f32,
    /// Moisture in 1000-hour fuels (fraction 0-1)
    moisture_1000h: f32,
    /// Weighted average moisture (fraction 0-1)
    average_moisture: f32,
}

impl FuelMoistureState {
    /// Create new moisture state with initial values for each timelag class
    pub(crate) fn new(
        moisture_1h: f32,
        moisture_10h: f32,
        moisture_100h: f32,
        moisture_1000h: f32,
    ) -> Self {
        let average = (moisture_1h + moisture_10h + moisture_100h + moisture_1000h) / 4.0;
        FuelMoistureState {
            moisture_1h,
            moisture_10h,
            moisture_100h,
            moisture_1000h,
            average_moisture: average,
        }
    }

    /// Get average moisture content
    pub(crate) fn average_moisture(&self) -> f32 {
        self.average_moisture
    }

    /// Update all moisture classes based on weather
    pub(crate) fn update(&mut self, fuel: &Fuel, temperature: f32, humidity: f32, dt_hours: f32) {
        // Determine if fuel is adsorbing (gaining) or desorbing (losing) moisture
        let emc = calculate_equilibrium_moisture(temperature, humidity, false);
        let is_adsorbing = self.average_moisture < emc;

        // Calculate equilibrium moisture (recalculate with correct direction)
        let emc = calculate_equilibrium_moisture(temperature, humidity, is_adsorbing);

        // Update each size class with its specific timelag
        self.moisture_1h =
            update_moisture_timelag(self.moisture_1h, emc, fuel.timelag_1h.0, dt_hours);

        self.moisture_10h =
            update_moisture_timelag(self.moisture_10h, emc, fuel.timelag_10h.0, dt_hours);

        self.moisture_100h =
            update_moisture_timelag(self.moisture_100h, emc, fuel.timelag_100h.0, dt_hours);

        self.moisture_1000h =
            update_moisture_timelag(self.moisture_1000h, emc, fuel.timelag_1000h.0, dt_hours);

        // Calculate weighted average
        self.average_moisture = calculate_weighted_moisture(
            self.moisture_1h,
            self.moisture_10h,
            self.moisture_100h,
            self.moisture_1000h,
            fuel.size_class_distribution.map(|f| f.0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equilibrium_moisture_calculation() {
        // Test at moderate conditions
        let emc = calculate_equilibrium_moisture(25.0, 50.0, false);

        // EMC should be between 5% and 20% for these conditions
        assert!(emc > 0.05 && emc < 0.20, "EMC was {}", emc);
    }

    #[test]
    fn test_equilibrium_moisture_humidity_effect() {
        // Higher humidity should give higher EMC
        let emc_low = calculate_equilibrium_moisture(20.0, 30.0, false);
        let emc_high = calculate_equilibrium_moisture(20.0, 70.0, false);

        assert!(emc_high > emc_low, "EMC should increase with humidity");
    }

    #[test]
    fn test_equilibrium_moisture_temperature_effect() {
        // Higher temperature should give lower EMC (drying effect)
        let emc_cool = calculate_equilibrium_moisture(10.0, 50.0, false);
        let emc_hot = calculate_equilibrium_moisture(40.0, 50.0, false);

        assert!(emc_hot < emc_cool, "EMC should decrease with temperature");
    }

    #[test]
    fn test_timelag_update_convergence() {
        let initial = 0.20;
        let target = 0.10;
        let timelag = 10.0; // 10 hours
        let dt = 1.0; // 1 hour

        let mut moisture = initial;

        // Update for several timelags
        for _ in 0..50 {
            moisture = update_moisture_timelag(moisture, target, timelag, dt);
        }

        // Should converge close to target after 50 hours (5 timelags)
        assert!(
            (moisture - target).abs() < 0.01,
            "Moisture was {} vs target {}",
            moisture,
            target
        );
    }

    #[test]
    fn test_fine_fuel_responds_faster() {
        let initial = 0.20;
        let target = 0.10;
        let dt = 1.0; // 1 hour

        // 1-hour fuel
        let moisture_1h = update_moisture_timelag(initial, target, 1.0, dt);

        // 100-hour fuel
        let moisture_100h = update_moisture_timelag(initial, target, 100.0, dt);

        // 1-hour fuel should change more in the same time
        assert!(
            (moisture_1h - target).abs() < (moisture_100h - target).abs(),
            "Fine fuels should respond faster"
        );
    }

    #[test]
    fn test_weighted_moisture() {
        let m1 = 0.10;
        let m10 = 0.15;
        let m100 = 0.20;
        let m1000 = 0.25;

        // Equal distribution
        let dist = [0.25, 0.25, 0.25, 0.25];
        let avg = calculate_weighted_moisture(m1, m10, m100, m1000, dist);

        // Should be average
        assert!((avg - 0.175).abs() < 0.01, "Weighted average was {}", avg);
    }

    #[test]
    fn test_moisture_state_update() {
        let fuel = Fuel::dry_grass(); // All 1-hour fuels
        let mut state = FuelMoistureState::new(0.15, 0.15, 0.15, 0.15);

        // Update with dry, hot conditions
        state.update(&fuel, 35.0, 20.0, 1.0);

        // Moisture should decrease
        assert!(
            state.average_moisture < 0.15,
            "Moisture should decrease in hot/dry conditions"
        );
    }

    #[test]
    fn test_diurnal_moisture_cycle() {
        let fuel = Fuel::eucalyptus_stringybark();
        let mut state = FuelMoistureState::new(0.12, 0.12, 0.12, 0.12);

        // Simulate day: hot and dry (should lose moisture)
        for _ in 0..12 {
            state.update(&fuel, 35.0, 25.0, 0.5); // 30-min steps
        }
        let day_moisture = state.average_moisture;

        // Simulate night: cool and humid (should gain moisture)
        for _ in 0..12 {
            state.update(&fuel, 15.0, 70.0, 0.5);
        }
        let night_moisture = state.average_moisture;

        // Should show diurnal pattern
        assert!(
            night_moisture > day_moisture,
            "Moisture should recover at night: day={}, night={}",
            day_moisture,
            night_moisture
        );
    }
}
