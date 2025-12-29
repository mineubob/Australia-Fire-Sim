//! Convective column dynamics for extreme fires.
//!
//! Implements Byram (1959) and Briggs (1975) plume rise models for calculating
//! convection column heights and updraft velocities from fire intensity.
//!
//! # Scientific Background
//!
//! Fire-generated convection columns are driven by buoyancy from the heat
//! released during combustion. The height and intensity of these columns
//! determine ember transport distances and influence local wind patterns.
//!
//! # References
//!
//! - Byram, G.M. (1959). "Combustion of forest fuels." Forest Fires: Control and Use.
//! - Briggs, G.A. (1975). "Plume rise predictions." NOAA Technical Memorandum.

/// Gravitational acceleration (m/s²).
pub const GRAVITY: f32 = 9.81;

/// Standard air density at sea level (kg/m³).
pub const AIR_DENSITY: f32 = 1.225;

/// Specific heat capacity of air at constant pressure (J/(kg·K)).
pub const SPECIFIC_HEAT_AIR: f32 = 1005.0;

/// Convective column from an intense fire.
///
/// Models the buoyant plume that rises from a fire, including height,
/// updraft velocity, and spatial extent. Used for ember transport
/// calculations and pyroconvection modeling.
#[derive(Clone, Debug)]
pub struct ConvectionColumn {
    /// Fire intensity driving the column (kW/m).
    pub intensity: f32,

    /// Calculated plume height (m).
    pub height: f32,

    /// Updraft velocity at base (m/s).
    pub updraft_velocity: f32,

    /// Column center position (x, y) in meters.
    pub position: (f32, f32),

    /// Column radius at base (m).
    pub base_radius: f32,
}

impl ConvectionColumn {
    /// Create a new convection column from fire parameters.
    ///
    /// # Arguments
    ///
    /// * `intensity` - Fire intensity (kW/m)
    /// * `fire_length` - Length of fire front (m)
    /// * `ambient_temp_k` - Ambient temperature (K)
    /// * `wind_speed_m_s` - Wind speed (m/s)
    /// * `position` - Column center position (x, y) in meters
    #[must_use]
    pub fn new(
        intensity: f32,
        fire_length: f32,
        ambient_temp_k: f32,
        wind_speed_m_s: f32,
        position: (f32, f32),
    ) -> Self {
        let height =
            Self::calculate_plume_height(intensity, fire_length, ambient_temp_k, wind_speed_m_s);
        let updraft_velocity = Self::calculate_updraft_velocity(intensity);
        // Base radius scales with intensity (empirical relationship)
        let base_radius = (intensity / 100.0).sqrt().clamp(10.0, 500.0);

        Self {
            intensity,
            height,
            updraft_velocity,
            position,
            base_radius,
        }
    }

    /// Calculate plume height using Briggs (1975) buoyancy-dominated formula.
    ///
    /// For buoyancy-dominated plumes in a crosswind:
    /// ```text
    /// z_max = 3.8 × F_b^0.6 / U
    /// F_b = (g × P) / (ρ × c_p × T_amb)
    /// ```
    ///
    /// Where:
    /// - P: Fire power (W) = I × L
    /// - U: Wind speed (m/s)
    /// - `F_b`: Buoyancy flux (m⁴/s³)
    ///
    /// # Arguments
    ///
    /// * `intensity_kw_m` - Fire intensity (kW/m)
    /// * `fire_length_m` - Fire front length (m)
    /// * `ambient_temp_k` - Ambient temperature (K)
    /// * `wind_speed_m_s` - Wind speed at 10m height (m/s)
    ///
    /// # Returns
    ///
    /// Plume height in meters, capped at 15000m (tropopause)
    #[must_use]
    pub fn calculate_plume_height(
        intensity_kw_m: f32,
        fire_length_m: f32,
        ambient_temp_k: f32,
        wind_speed_m_s: f32,
    ) -> f32 {
        // Convert intensity from kW/m to W total
        let power_watts = intensity_kw_m * 1000.0 * fire_length_m;

        // Calculate buoyancy flux (m⁴/s³)
        // F_b = (g × P) / (ρ × c_p × T_amb)
        let buoyancy_flux =
            (GRAVITY * power_watts) / (AIR_DENSITY * SPECIFIC_HEAT_AIR * ambient_temp_k);

        // Minimum wind speed to avoid division by zero
        let effective_wind = wind_speed_m_s.max(0.5);

        // Briggs (1975) buoyancy-dominated plume rise
        // z = 3.8 × F_b^0.6 / U
        let height = 3.8 * buoyancy_flux.powf(0.6) / effective_wind;

        // Cap at tropopause height (~15km)
        height.min(15000.0)
    }

    /// Calculate updraft velocity at plume base.
    ///
    /// Uses simplified convective velocity scale:
    /// ```text
    /// w = sqrt(2 × g × H × ΔT / T)
    /// ```
    ///
    /// Temperature excess scales with fire intensity (empirical).
    ///
    /// # Arguments
    ///
    /// * `intensity_kw_m` - Fire intensity (kW/m)
    ///
    /// # Returns
    ///
    /// Updraft velocity in m/s (typically 5-30 m/s)
    #[must_use]
    pub fn calculate_updraft_velocity(intensity_kw_m: f32) -> f32 {
        // Temperature excess scales with intensity, capped at 300K
        let delta_t = 300.0 * (intensity_kw_m / 100_000.0).min(1.0);

        // Reference height for convective scale (100m)
        let reference_height = 100.0;

        // Ambient temperature (K)
        let t_ambient = 300.0;

        // w = sqrt(2 × g × H × ΔT / T)
        let velocity = (2.0 * GRAVITY * reference_height * delta_t / t_ambient).sqrt();

        // Clamp to physical range (5-50 m/s)
        velocity.clamp(0.0, 50.0)
    }

    /// Check if this column could generate a pyroCb.
    ///
    /// `PyroCb` formation requires:
    /// - Very tall plume (> 8000m)
    /// - High intensity (> 50,000 kW/m)
    #[must_use]
    pub fn can_generate_pyrocb(&self) -> bool {
        self.height > 8000.0 && self.intensity > 50_000.0
    }

    /// Calculate entrainment velocity at a given distance from column center.
    ///
    /// Air is drawn toward the column base due to the updraft.
    /// Entrainment velocity scales as: `v_entrain` ≈ 0.1 × w^(1/3) × (R/r)
    ///
    /// # Arguments
    ///
    /// * `distance` - Distance from column center (m)
    ///
    /// # Returns
    ///
    /// Entrainment velocity in m/s (toward column)
    #[must_use]
    pub fn entrainment_velocity_at(&self, distance: f32) -> f32 {
        if distance < self.base_radius || distance > self.base_radius * 10.0 {
            return 0.0;
        }

        // v_entrain ≈ 0.1 × w^(1/3) × (R/r)
        0.1 * self.updraft_velocity.powf(0.33) * (self.base_radius / distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that plume height matches Briggs (1975) for typical fire.
    #[test]
    fn plume_height_byram() {
        // 10,000 kW/m intensity, 100m fire line, 300K ambient, 5 m/s wind
        let height = ConvectionColumn::calculate_plume_height(10_000.0, 100.0, 300.0, 5.0);

        // Briggs formula produces moderate heights for typical fires
        // Higher plumes require much larger fires (higher power or longer fronts)
        assert!(height > 100.0, "Plume height {height}m should be > 100m");
        assert!(height < 2000.0, "Plume height {height}m should be < 2000m");

        // Extreme fire should produce tall plume
        let extreme_height =
            ConvectionColumn::calculate_plume_height(100_000.0, 1000.0, 300.0, 3.0);
        assert!(
            extreme_height > 3000.0,
            "Extreme fire plume {extreme_height}m should be > 3000m"
        );
    }

    /// Test that higher intensity produces taller plume.
    #[test]
    fn plume_height_scales_with_intensity() {
        let low_intensity = ConvectionColumn::calculate_plume_height(1_000.0, 100.0, 300.0, 5.0);
        let high_intensity = ConvectionColumn::calculate_plume_height(50_000.0, 100.0, 300.0, 5.0);

        assert!(
            high_intensity > low_intensity * 2.0,
            "High intensity plume {high_intensity}m should be much taller than low {low_intensity}m"
        );
    }

    /// Test that higher wind reduces plume height (bent over plume).
    #[test]
    fn plume_height_decreases_with_wind() {
        let calm = ConvectionColumn::calculate_plume_height(10_000.0, 100.0, 300.0, 1.0);
        let windy = ConvectionColumn::calculate_plume_height(10_000.0, 100.0, 300.0, 10.0);

        assert!(
            calm > windy,
            "Calm plume {calm}m should be taller than windy plume {windy}m"
        );
    }

    /// Test updraft velocity is in reasonable range.
    #[test]
    fn updraft_velocity_range() {
        // Low intensity
        let low_updraft = ConvectionColumn::calculate_updraft_velocity(1_000.0);
        assert!(
            (0.0..20.0).contains(&low_updraft),
            "Low intensity updraft {low_updraft} m/s should be 0-20 m/s"
        );

        // High intensity
        let high_updraft = ConvectionColumn::calculate_updraft_velocity(100_000.0);
        assert!(
            (10.0..=50.0).contains(&high_updraft),
            "High intensity updraft {high_updraft} m/s should be 10-50 m/s"
        );
    }

    /// Test column creation.
    #[test]
    fn column_creation() {
        let column = ConvectionColumn::new(10_000.0, 100.0, 300.0, 5.0, (500.0, 500.0));

        assert!((column.intensity - 10_000.0).abs() < f32::EPSILON);
        assert!(column.height > 0.0);
        assert!(column.updraft_velocity > 0.0);
        assert!(column.base_radius > 0.0);
    }

    /// Test entrainment velocity.
    #[test]
    fn entrainment_velocity_decreases_with_distance() {
        let column = ConvectionColumn::new(50_000.0, 200.0, 300.0, 3.0, (500.0, 500.0));

        let near = column.entrainment_velocity_at(column.base_radius * 2.0);
        let far = column.entrainment_velocity_at(column.base_radius * 5.0);

        assert!(
            near > far,
            "Near entrainment {near} should exceed far {far}"
        );
    }

    /// Test pyroCb potential detection.
    #[test]
    fn pyrocb_potential() {
        // Moderate fire - no pyroCb
        let moderate = ConvectionColumn::new(10_000.0, 100.0, 300.0, 5.0, (0.0, 0.0));
        assert!(!moderate.can_generate_pyrocb());

        // Extreme fire - potential pyroCb
        let extreme = ConvectionColumn::new(100_000.0, 500.0, 300.0, 2.0, (0.0, 0.0));
        assert!(extreme.can_generate_pyrocb());
    }
}
