//! Downdraft and gust front dynamics.
//!
//! Implements Byers & Braham (1949) downdraft physics for modeling
//! the dangerous wind gusts that occur when a pyroCb collapses.
//!
//! # Scientific Background
//!
//! When a convective column or pyroCb collapses, it generates a downdraft
//! that spreads outward at the surface as a gust front. These can cause
//! sudden, dangerous wind shifts that create erratic fire behavior.
//!
//! # References
//!
//! - Byers, H.R. & Braham, R.R. (1949). "The Thunderstorm." U.S. Weather Bureau.

use super::GRAVITY;

/// Downdraft from pyroCb or convective column collapse.
///
/// Models the downward air motion and subsequent surface outflow
/// that creates dangerous gust fronts during extreme fire events.
#[derive(Clone, Debug)]
pub struct Downdraft {
    /// Center position (x, y) in meters.
    pub position: (f32, f32),

    /// Vertical velocity (m/s, negative = downward).
    pub vertical_velocity: f32,

    /// Radius of influence (m).
    pub radius: f32,

    /// Outflow velocity at surface (m/s).
    pub outflow_velocity: f32,
}

impl Downdraft {
    /// Create a downdraft from pyroCb collapse.
    ///
    /// Uses Byers & Braham (1949) downdraft velocity formula:
    /// ```text
    /// w_down = -sqrt(2 × g × H × |Δθ| / θ_env)
    /// ```
    ///
    /// Where:
    /// - H: Downdraft depth (taken as half column height)
    /// - Δθ: Potential temperature deficit from evaporative cooling
    ///
    /// # Arguments
    ///
    /// * `position` - Center position (x, y) in meters
    /// * `column_height_m` - Original convection column height (m)
    /// * `ambient_temp_k` - Ambient temperature (K)
    /// * `precipitation_loading_kg_m3` - Precipitation mass loading (kg/m³)
    ///
    /// # Returns
    ///
    /// New downdraft with calculated velocities
    #[must_use]
    pub fn from_pyrocb(
        position: (f32, f32),
        column_height_m: f32,
        ambient_temp_k: f32,
        precipitation_loading_kg_m3: f32,
    ) -> Self {
        // Downdraft depth is approximately half the column height
        let downdraft_depth = column_height_m * 0.5;

        // Temperature deficit from evaporative cooling
        // Typically 5-15K for moderate to heavy precipitation
        let delta_theta = 10.0 * precipitation_loading_kg_m3.min(1.5);

        // Byers & Braham downdraft velocity (negative = downward)
        // w_down = -sqrt(2 × g × H × |Δθ| / θ_env)
        let w_down = if delta_theta > 0.0 {
            -(2.0 * GRAVITY * downdraft_depth * delta_theta / ambient_temp_k).sqrt()
        } else {
            0.0
        };

        // Outflow velocity from momentum conservation (typically 80% of |w_down|)
        let outflow = (-w_down * 0.8).max(5.0);

        // Initial radius based on column scale
        let initial_radius = (column_height_m * 0.1).clamp(200.0, 2000.0);

        Self {
            position,
            vertical_velocity: w_down,
            radius: initial_radius,
            outflow_velocity: outflow,
        }
    }

    /// Create a downdraft with specified parameters.
    ///
    /// # Arguments
    ///
    /// * `position` - Center position (x, y) in meters
    /// * `vertical_velocity` - Downdraft speed (m/s, negative)
    /// * `radius` - Initial radius (m)
    /// * `outflow_velocity` - Surface outflow speed (m/s)
    #[must_use]
    pub fn new(
        position: (f32, f32),
        vertical_velocity: f32,
        radius: f32,
        outflow_velocity: f32,
    ) -> Self {
        Self {
            position,
            vertical_velocity,
            radius,
            outflow_velocity,
        }
    }

    /// Update downdraft spreading over time.
    ///
    /// The radius expands as the outflow spreads, while velocities
    /// decay due to friction and mixing.
    ///
    /// # Arguments
    ///
    /// * `dt_seconds` - Time step in seconds
    pub fn update(&mut self, dt_seconds: f32) {
        // Expand radius as downdraft spreads
        self.radius += self.outflow_velocity * dt_seconds * 0.5;

        // Decay outflow velocity (exponential decay, ~1% per second)
        let decay_rate = 0.99_f32;
        self.outflow_velocity *= decay_rate.powf(dt_seconds);

        // Also decay vertical velocity
        self.vertical_velocity *= decay_rate.powf(dt_seconds);
    }

    /// Calculate wind effect at a position (returns (u, v) in m/s).
    ///
    /// The outflow is radially symmetric, directed away from the
    /// downdraft center, with velocity decreasing with distance.
    ///
    /// # Arguments
    ///
    /// * `position` - Query position (x, y) in meters
    ///
    /// # Returns
    ///
    /// Wind modification (u, v) in m/s
    #[must_use]
    pub fn wind_effect_at(&self, position: (f32, f32)) -> (f32, f32) {
        let dx = position.0 - self.position.0;
        let dy = position.1 - self.position.1;
        let distance = (dx * dx + dy * dy).sqrt();

        // No effect outside radius or at center
        if distance > self.radius || distance < 1.0 {
            return (0.0, 0.0);
        }

        // Normalize direction (radial outflow)
        let dir_x = dx / distance;
        let dir_y = dy / distance;

        // Velocity profile: maximum at some distance from center, zero at edge
        // Using a parabolic profile: strength = outflow × (1 - (r/R)²) × 4 × (r/R)
        // This gives zero at center, maximum at r = R/2, zero at edge
        let normalized_dist = distance / self.radius;
        let strength = self.outflow_velocity * 4.0 * normalized_dist * (1.0 - normalized_dist);

        (dir_x * strength, dir_y * strength)
    }

    /// Check if this downdraft has effectively dissipated.
    ///
    /// Returns true if outflow velocity drops below 1 m/s.
    #[must_use]
    pub fn is_dissipated(&self) -> bool {
        self.outflow_velocity < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test downdraft velocity is in reasonable range.
    #[test]
    fn downdraft_velocity_range() {
        let downdraft = Downdraft::from_pyrocb((0.0, 0.0), 10_000.0, 288.0, 0.5);

        // Typical downdraft velocities are 10-30 m/s
        let speed = downdraft.vertical_velocity.abs();
        assert!(
            speed > 5.0 && speed < 50.0,
            "Downdraft velocity {speed} m/s should be in reasonable range"
        );
    }

    /// Test that downdraft radius increases over time.
    #[test]
    fn downdraft_outflow_spreading() {
        let mut downdraft = Downdraft::from_pyrocb((500.0, 500.0), 8000.0, 290.0, 0.3);

        let initial_radius = downdraft.radius;
        downdraft.update(60.0); // 1 minute

        assert!(
            downdraft.radius > initial_radius,
            "Radius should expand: {initial_radius} → {}",
            downdraft.radius
        );
    }

    /// Test that outflow is radially symmetric.
    #[test]
    fn wind_effect_radial() {
        let downdraft = Downdraft::new((0.0, 0.0), -20.0, 500.0, 15.0);

        // Test at different angles
        let (u_east, v_east) = downdraft.wind_effect_at((250.0, 0.0));
        let (u_north, v_north) = downdraft.wind_effect_at((0.0, 250.0));

        // East point should have positive u (eastward outflow)
        assert!(u_east > 0.0, "East outflow u={u_east} should be positive");
        assert!(
            v_east.abs() < 0.01,
            "East outflow v={v_east} should be near zero"
        );

        // North point should have positive v (northward outflow)
        assert!(
            v_north > 0.0,
            "North outflow v={v_north} should be positive"
        );
        assert!(
            u_north.abs() < 0.01,
            "North outflow u={u_north} should be near zero"
        );
    }

    /// Test no wind effect outside radius.
    #[test]
    fn wind_effect_outside_radius() {
        let downdraft = Downdraft::new((0.0, 0.0), -20.0, 500.0, 15.0);

        let (u, v) = downdraft.wind_effect_at((600.0, 600.0));
        assert!(
            u.abs() < f32::EPSILON && v.abs() < f32::EPSILON,
            "No wind effect outside radius: ({u}, {v})"
        );
    }

    /// Test dissipation check.
    #[test]
    fn dissipation_check() {
        let mut downdraft = Downdraft::new((0.0, 0.0), -20.0, 500.0, 15.0);
        assert!(!downdraft.is_dissipated());

        // Update for a long time to decay
        for _ in 0..1000 {
            downdraft.update(1.0);
        }
        assert!(downdraft.is_dissipated());
    }
}
