//! Pyrocumulus cloud physics (for future integration)
#![allow(dead_code)]

//! Pyrocumulus Cloud Formation and Dynamics
//!
//! Implements fire-generated cloud physics for extreme fire behavior modeling.
//!
//! # Scientific References
//!
//! - Fromm, M., et al. (2010). "Pyro-cumulonimbus injection of smoke to the
//!   stratosphere." Annales Geophysicae, 28(4), 937-963.
//! - Tory, K.J., & Thurston, W. (2015). "Pyrocumulonimbus: A review."
//!   Current Pollution Reports, 1(4), 234-245.
//! - Briggs, G.A. (1975). "Plume rise predictions." NOAA/ATDL.

use super::AtmosphericProfile;
use crate::core_types::element::Vec3;
use serde::{Deserialize, Serialize};

/// Fire-generated pyrocumulus cloud
///
/// Pyrocumulus (pyroCu) clouds form when intense fires generate
/// enough heat to lift air above the condensation level. These
/// clouds can:
///
/// - Generate their own weather (rain, lightning, downbursts)
/// - Create extreme fire behavior through wind modification
/// - Loft embers to extreme heights (30+ km in pyroCb)
/// - Cause fire tornadoes through rotation
///
/// # Lifecycle
///
/// 1. **Formation**: Fire intensity > 10 MW/m, unstable atmosphere
/// 2. **Development**: Cloud grows vertically, condensation begins
/// 3. **Maturity**: Maximum extent, possible precipitation
/// 4. **Decay**: Fire intensity drops, cloud dissipates
///
/// # Fire Feedback Effects
///
/// - **Inflow**: Strong surface winds toward fire (feeds combustion)
/// - **Outflow**: Upper-level divergence creates erratic winds
/// - **Ember lofting**: Updrafts carry embers to extreme heights
/// - **Moisture**: Precipitation may suppress fire (rare)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PyrocumulusCloud {
    /// Cloud base position (x, y, z in meters)
    pub(crate) position: Vec3,

    /// Cloud base altitude above ground level (meters)
    pub(crate) base_altitude: f32,

    /// Cloud top altitude above ground level (meters)
    pub(crate) top_altitude: f32,

    /// Horizontal extent radius (meters)
    pub(crate) horizontal_extent: f32,

    /// Maximum updraft velocity (m/s)
    /// Typical: 10-30 m/s, extreme pyroCb: 50+ m/s
    pub(crate) updraft_velocity: f32,

    /// Condensation rate (kg/s)
    pub(crate) condensation_rate: f32,

    /// Whether cloud is producing precipitation
    pub(crate) precipitation: bool,

    /// Amount of precipitation (mm/hr)
    pub(crate) precipitation_rate: f32,

    /// Inflow/outflow wind modification at surface (m/s)
    pub(crate) wind_modification: Vec3,

    /// Humidity increase in fire vicinity (fraction)
    pub(crate) humidity_increase: f32,

    /// Rotation detected (fire tornado risk)
    pub(crate) rotation_detected: bool,

    /// Rotation velocity if present (m/s tangential)
    pub(crate) rotation_velocity: f32,

    /// Number of lightning strikes generated
    pub(crate) lightning_strikes: u32,

    /// Cloud development stage (0-1)
    /// 0.0: Just forming
    /// 0.5: Developing
    /// 1.0: Mature
    pub(crate) development_stage: f32,

    /// Cloud age (seconds since formation)
    pub(crate) age: f32,

    /// Fire intensity that generated this cloud (kW/m)
    pub(crate) source_intensity: f32,
}

impl PyrocumulusCloud {
    /// Attempt to form a pyrocumulus cloud from a high-intensity fire
    ///
    /// # Parameters
    /// - `fire_position`: Center of fire
    /// - `fire_intensity`: Fireline intensity (kW/m)
    /// - `atmosphere`: Current atmospheric profile
    /// - `ambient_humidity`: Surface relative humidity (0-1)
    ///
    /// # Returns
    /// Some(cloud) if conditions support formation, None otherwise
    ///
    /// # Scientific Basis
    ///
    /// Pyrocumulus formation requires:
    /// 1. Fire intensity > ~10 MW/m (10,000 kW/m)
    /// 2. Unstable atmosphere (negative LI)
    /// 3. Adequate mixing height
    /// 4. No strong capping inversion
    pub fn try_form(
        fire_position: Vec3,
        fire_intensity: f32,
        atmosphere: &AtmosphericProfile,
        _ambient_humidity: f32,
    ) -> Option<Self> {
        // Check if conditions support formation
        let (can_form, _intensity_factor) = atmosphere.pyrocumulus_potential(fire_intensity);

        if !can_form {
            return None;
        }

        // Calculate cloud base (Lifting Condensation Level)
        // LCL ≈ 125 × (T - Td) meters
        let lcl = Self::calculate_lcl(atmosphere.surface_temperature, atmosphere.surface_dewpoint);

        // Calculate initial updraft velocity using Briggs (1975) plume rise
        let heat_flux = fire_intensity * 50.0; // Approximate conversion kW/m to total heat
        let updraft = Self::calculate_updraft_velocity(heat_flux, atmosphere);

        // Cloud top depends on instability (CAPE)
        let cape = atmosphere.estimate_cape();
        let cloud_depth = if cape > 2000.0 {
            5000.0 // Deep convection
        } else if cape > 1000.0 {
            3000.0 // Moderate convection
        } else {
            1500.0 // Shallow
        };

        let top_altitude = lcl + cloud_depth;

        // Horizontal extent based on intensity
        let horizontal_extent = (fire_intensity / 1000.0).sqrt() * 100.0;

        Some(PyrocumulusCloud {
            position: fire_position,
            base_altitude: lcl,
            top_altitude,
            horizontal_extent,
            updraft_velocity: updraft,
            condensation_rate: 0.0,
            precipitation: false,
            precipitation_rate: 0.0,
            wind_modification: Vec3::zeros(),
            humidity_increase: 0.0,
            rotation_detected: false,
            rotation_velocity: 0.0,
            lightning_strikes: 0,
            development_stage: 0.0,
            age: 0.0,
            source_intensity: fire_intensity,
        })
    }

    /// Calculate Lifting Condensation Level (LCL)
    ///
    /// LCL ≈ 125 × (T - Td) meters
    ///
    /// This is the altitude where rising air reaches saturation.
    fn calculate_lcl(temperature: f32, dewpoint: f32) -> f32 {
        let lcl = 125.0 * (temperature - dewpoint);
        lcl.clamp(500.0, 5000.0) // Physical limits
    }

    /// Calculate updraft velocity using Briggs (1975) plume rise
    ///
    /// For buoyancy-dominated plumes:
    /// w = (F / (π × ρ))^(1/3)
    ///
    /// Where F is the buoyancy flux.
    fn calculate_updraft_velocity(heat_flux_kw: f32, atmosphere: &AtmosphericProfile) -> f32 {
        const AIR_DENSITY: f32 = 1.225; // kg/m³
        const SPECIFIC_HEAT_AIR: f32 = 1.005; // kJ/(kg·K)
        const GRAVITY: f32 = 9.81;

        // Buoyancy flux F = g × Q / (ρ × cp × T)
        let temp_k = atmosphere.surface_temperature + 273.15;
        let buoyancy_flux = GRAVITY * heat_flux_kw / (AIR_DENSITY * SPECIFIC_HEAT_AIR * temp_k);

        // Updraft velocity (simplified)
        let w = (buoyancy_flux).powf(1.0 / 3.0) * 2.5;

        // Add instability enhancement
        let instability_boost = if atmosphere.lifted_index < -4.0 {
            1.5
        } else if atmosphere.lifted_index < -2.0 {
            1.2
        } else {
            1.0
        };

        (w * instability_boost).clamp(5.0, 50.0)
    }

    /// Update cloud dynamics over time
    ///
    /// # Parameters
    /// - `dt`: Time step (seconds)
    /// - `fire_intensity`: Current fire intensity (kW/m)
    /// - `atmosphere`: Current atmospheric profile
    pub fn update(&mut self, dt: f32, fire_intensity: f32, atmosphere: &AtmosphericProfile) {
        self.age += dt;

        // 1. Development stage
        // Takes ~10 minutes (600s) to fully develop
        self.development_stage = (self.age / 600.0).min(1.0);

        // 2. Updraft evolution
        // Decays due to entrainment, but maintained by fire
        let fire_support = fire_intensity / self.source_intensity;
        let entrainment_decay = 0.002 * dt; // ~2% per second
        self.updraft_velocity *= 1.0 - entrainment_decay + (fire_support - 1.0) * 0.001;
        self.updraft_velocity = self.updraft_velocity.clamp(0.0, 50.0);

        // 3. Vertical extent
        // Cloud top rises during development, then stabilizes
        if self.development_stage < 1.0 {
            self.top_altitude += self.updraft_velocity * 0.5 * dt;
        }

        // 4. Horizontal spread
        self.horizontal_extent += self.updraft_velocity * 0.05 * dt;

        // 5. Condensation and precipitation
        // Starts when cloud depth exceeds ~3 km
        let cloud_depth = self.top_altitude - self.base_altitude;
        if cloud_depth > 3000.0 && self.development_stage > 0.5 {
            // Condensation rate proportional to updraft and cloud size
            self.condensation_rate = self.updraft_velocity * self.horizontal_extent * 0.001;

            // Precipitation threshold
            if self.condensation_rate > 1000.0 {
                self.precipitation = true;
                // Light rain: 1-10 mm/hr, heavy: 10-50 mm/hr
                self.precipitation_rate = (self.condensation_rate / 500.0).clamp(0.0, 50.0);
            }
        }

        // 6. Wind modification (inflow at surface)
        self.calculate_wind_modification(atmosphere);

        // 7. Humidity increase
        if self.precipitation {
            self.humidity_increase = 0.1 + self.precipitation_rate * 0.01;
        } else {
            self.humidity_increase = 0.05 * self.development_stage;
        }

        // 8. Fire tornado risk assessment
        self.check_fire_tornado_risk(atmosphere, fire_intensity);

        // 9. Lightning (for deep convection / pyroCb)
        if cloud_depth > 5000.0 && self.updraft_velocity > 20.0 {
            // Probability of lightning increases with cloud depth and updraft
            let lightning_prob = (cloud_depth - 5000.0) / 10000.0 * (self.updraft_velocity / 30.0);
            if lightning_prob > 0.0 && rand::random::<f32>() < lightning_prob * dt / 60.0 {
                self.lightning_strikes += 1;
            }
        }
    }

    /// Calculate surface wind modification from convective column
    fn calculate_wind_modification(&mut self, atmosphere: &AtmosphericProfile) {
        // Inflow velocity toward cloud center
        // Proportional to updraft strength
        let inflow_speed = self.updraft_velocity * 0.3 * self.development_stage;

        // Direction: inward radially (simplified as single vector)
        // In reality, this would vary with position relative to cloud
        self.wind_modification = Vec3::new(
            inflow_speed * 0.5,
            inflow_speed * 0.5,
            inflow_speed * 0.1, // Slight upward component
        );

        // Add shear-induced asymmetry
        if atmosphere.wind_shear > 5.0 {
            self.wind_modification.x += atmosphere.wind_shear * 0.1;
        }
    }

    /// Check for fire tornado (fire whirl) conditions
    fn check_fire_tornado_risk(&mut self, atmosphere: &AtmosphericProfile, fire_intensity: f32) {
        let tornado_risk = atmosphere.fire_tornado_risk(fire_intensity);

        // Additional factors from cloud dynamics
        let cloud_factor = if self.updraft_velocity > 20.0 {
            1.5
        } else if self.updraft_velocity > 10.0 {
            1.2
        } else {
            1.0
        };

        let adjusted_risk = tornado_risk * cloud_factor;

        // Rotation threshold
        if adjusted_risk > 0.5 {
            self.rotation_detected = true;
            // Tangential velocity estimate (NIST fire whirl research)
            // Typical range: 10-50 m/s
            self.rotation_velocity = (adjusted_risk * 40.0).clamp(10.0, 50.0);
        } else {
            self.rotation_detected = false;
            self.rotation_velocity = 0.0;
        }
    }

    /// Calculate ember lofting height enhancement from cloud updraft
    ///
    /// # Returns
    /// Multiplier for standard ember lofting height
    pub fn ember_lofting_multiplier(&self) -> f32 {
        // Strong updrafts can loft embers 2-5x higher
        let base_multiplier = 1.0 + (self.updraft_velocity / 20.0);
        base_multiplier.clamp(1.0, 5.0)
    }

    /// Check if cloud is still active
    #[expect(dead_code)]
    pub fn is_active(&self) -> bool {
        self.updraft_velocity > 2.0
    }

    /// Get cloud type classification
    pub fn cloud_type(&self) -> &'static str {
        let depth = self.top_altitude - self.base_altitude;

        if self.lightning_strikes > 0 || depth > 10000.0 {
            "Pyrocumulonimbus (pyroCb)"
        } else if depth > 5000.0 {
            "Deep Pyrocumulus"
        } else if depth > 2000.0 {
            "Moderate Pyrocumulus"
        } else {
            "Cumulus Flammagenitus"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyrocumulus_formation_conditions() {
        // Create unstable atmosphere
        let atmosphere = AtmosphericProfile {
            lifted_index: -4.0,
            mixing_height: 3000.0,
            inversion_strength: 0.0,
            surface_temperature: 35.0,
            surface_dewpoint: 15.0,
            temp_850: 20.0,
            temp_500: -15.0,
            ..Default::default()
        };

        // High intensity should form cloud
        let cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 25_000.0, &atmosphere, 0.3);

        assert!(
            cloud.is_some(),
            "High intensity + unstable should form pyroCu"
        );

        // Low intensity should not
        let no_cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 3_000.0, &atmosphere, 0.3);

        assert!(no_cloud.is_none(), "Low intensity should not form pyroCu");
    }

    #[test]
    fn test_stable_atmosphere_blocks_formation() {
        // Create stable atmosphere
        let atmosphere = AtmosphericProfile {
            lifted_index: 4.0, // Positive = stable
            mixing_height: 1000.0,
            inversion_strength: 5.0, // Strong inversion
            ..Default::default()
        };

        // Even high intensity shouldn't form in stable conditions
        let cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 50_000.0, &atmosphere, 0.3);

        assert!(cloud.is_none(), "Stable atmosphere should prevent pyroCu");
    }

    #[test]
    fn test_cloud_development() {
        let atmosphere = AtmosphericProfile {
            lifted_index: -4.0,
            mixing_height: 3000.0,
            ..Default::default()
        };

        let mut cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 25_000.0, &atmosphere, 0.3)
                .unwrap();

        let initial_top = cloud.top_altitude;

        // Simulate 5 minutes of development
        for _ in 0..300 {
            cloud.update(1.0, 25_000.0, &atmosphere);
        }

        assert!(cloud.development_stage > 0.4, "Should be developing");
        assert!(
            cloud.top_altitude >= initial_top,
            "Cloud should grow upward"
        );
    }

    #[test]
    fn test_ember_lofting_enhancement() {
        let atmosphere = AtmosphericProfile {
            lifted_index: -5.0,
            ..Default::default()
        };

        let mut cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 30_000.0, &atmosphere, 0.3)
                .unwrap();

        // Allow some development
        for _ in 0..60 {
            cloud.update(1.0, 30_000.0, &atmosphere);
        }

        let multiplier = cloud.ember_lofting_multiplier();
        assert!(multiplier > 1.0, "Should enhance ember lofting");
    }

    #[test]
    fn test_lcl_calculation() {
        // Warm, humid air has low LCL
        let lcl_humid = PyrocumulusCloud::calculate_lcl(30.0, 22.0);

        // Warm, dry air has high LCL
        let lcl_dry = PyrocumulusCloud::calculate_lcl(30.0, 5.0);

        assert!(lcl_humid < lcl_dry, "Humid air should have lower LCL");
        assert!(lcl_humid > 500.0, "LCL should be reasonable");
        assert!(lcl_dry < 5000.0, "LCL should be reasonable");
    }

    #[test]
    fn test_cloud_type_classification() {
        let atmosphere = AtmosphericProfile {
            lifted_index: -6.0,
            ..Default::default()
        };

        let mut cloud = PyrocumulusCloud::try_form(
            Vec3::new(0.0, 0.0, 0.0),
            50_000.0, // Very high intensity
            &atmosphere,
            0.3,
        )
        .unwrap();

        // Force deep development
        cloud.top_altitude = 12000.0;
        cloud.lightning_strikes = 1;

        assert_eq!(cloud.cloud_type(), "Pyrocumulonimbus (pyroCb)");
    }

    #[test]
    fn test_fire_tornado_detection() {
        let high_shear = AtmosphericProfile {
            lifted_index: -4.0,
            wind_shear: 20.0, // High shear
            ..Default::default()
        };

        let mut cloud =
            PyrocumulusCloud::try_form(Vec3::new(0.0, 0.0, 0.0), 40_000.0, &high_shear, 0.3)
                .unwrap();

        cloud.updraft_velocity = 25.0; // Strong updraft

        cloud.check_fire_tornado_risk(&high_shear, 40_000.0);

        // Should detect rotation with high shear and intensity
        assert!(
            cloud.rotation_detected || cloud.rotation_velocity > 0.0,
            "Should detect fire tornado risk with high shear"
        );
    }
}
