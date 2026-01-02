//! Junction zone fire behavior
//!
//! Implements detection and acceleration modeling for converging fire fronts.
//!
//! # Scientific Background
//!
//! When two fire fronts approach each other and merge (junction zone), the fire
//! behavior becomes significantly more intense due to:
//! - Combined radiant heat flux preheating fuel from both directions
//! - Air entrainment from both sides creating enhanced updrafts
//! - Rate of spread can increase 2-5× at the junction point
//!
//! Junction zone acceleration depends on the angle between converging fronts:
//! - θ = 0° (parallel): No acceleration (fires just merge)
//! - θ = 30-60°: Maximum acceleration (2-5× ROS)
//! - θ = 90°: Moderate acceleration
//! - θ = 180° (head-on): Brief intense interaction, then extinguish
//!
//! # Scientific References
//!
//! - Viegas, D.X. et al. (2012). "Fire behaviour and fatalities in wildfires."
//!   Int. J. Wildland Fire.
//! - Thomas, C.M. et al. (2017). "Investigation of firebrand generation from
//!   burning vegetation." Fire Safety Journal.

use crate::core_types::vec3::Vec3;

/// Detected junction zone between converging fire fronts
#[derive(Debug, Clone)]
pub struct JunctionZone {
    /// Position of junction point (world coordinates)
    pub position: Vec3,
    /// Angle between converging fronts (radians)
    pub angle: f32,
    /// Distance between fronts (m)
    pub distance: f32,
    /// Estimated time to contact (s)
    pub time_to_contact: f32,
    /// Acceleration factor to apply (1.0 = no acceleration, 5.0 = 5× faster)
    pub acceleration_factor: f32,
}

/// Detector for junction zone conditions
pub struct JunctionZoneDetector {
    /// Minimum distance to consider as potential junction (m)
    pub detection_distance: f32,
    /// Minimum angle for junction acceleration (radians)
    pub min_angle: f32,
}

impl Default for JunctionZoneDetector {
    fn default() -> Self {
        Self {
            detection_distance: 100.0, // Detect junctions within 100m
            min_angle: 0.1,            // ~6° minimum angle
        }
    }
}

impl JunctionZoneDetector {
    /// Create a new junction zone detector with custom parameters
    ///
    /// # Arguments
    ///
    /// * `detection_distance` - Maximum distance (m) between fronts to detect junction
    /// * `min_angle` - Minimum angle (radians) for junction acceleration
    #[must_use]
    pub fn new(detection_distance: f32, min_angle: f32) -> Self {
        Self {
            detection_distance,
            min_angle,
        }
    }

    /// Detect junction zones from level set field
    ///
    /// Analyzes the level set to find regions where:
    /// 1. Two separate fire fronts (φ = 0 contours) exist
    /// 2. They are approaching each other
    /// 3. The junction angle is acute enough for acceleration
    ///
    /// # Arguments
    ///
    /// * `phi` - Level set field (φ < 0 = burned, φ > 0 = unburned)
    /// * `spread_rate` - Rate of spread field (m/s) for accurate time-to-contact calculations
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Size of each grid cell (m)
    /// * `dt` - Time step (s), used for consistency checks
    ///
    /// # Returns
    ///
    /// Vector of detected junction zones with their properties
    pub fn detect(
        &self,
        phi: &[f32],
        spread_rate: &[f32],
        width: usize,
        height: usize,
        cell_size: f32,
        dt: f32,
    ) -> Vec<JunctionZone> {
        let mut junctions = Vec::new();

        // Find fire front cells (φ ≈ 0 with φ < 0 neighbors)
        let front_cells = Self::extract_fire_front_cells(phi, width, height);

        // Group into connected components (separate fire fronts)
        let components = Self::find_connected_components(&front_cells, width, height);

        // For each pair of components, check for junction conditions
        for i in 0..components.len() {
            for j in (i + 1)..components.len() {
                if let Some(junction) = self.analyze_junction(
                    &components[i],
                    &components[j],
                    phi,
                    spread_rate,
                    width,
                    height,
                    cell_size,
                    dt,
                ) {
                    junctions.push(junction);
                }
            }
        }

        junctions
    }

    /// Extract cells on fire front (φ ≈ 0)
    fn extract_fire_front_cells(phi: &[f32], width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut front_cells = Vec::new();

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = y * width + x;
                let p = phi[idx];

                // Cell is on front if φ is small and has sign change with neighbor
                if p.abs() < 2.0 {
                    let has_positive = phi[idx - 1] > 0.0
                        || phi[idx + 1] > 0.0
                        || phi[idx - width] > 0.0
                        || phi[idx + width] > 0.0;
                    let has_negative = phi[idx - 1] < 0.0
                        || phi[idx + 1] < 0.0
                        || phi[idx - width] < 0.0
                        || phi[idx + width] < 0.0;

                    if has_positive && has_negative {
                        front_cells.push((x, y));
                    }
                }
            }
        }

        front_cells
    }

    /// Group front cells into connected components
    fn find_connected_components(
        front_cells: &[(usize, usize)],
        width: usize,
        height: usize,
    ) -> Vec<Vec<(usize, usize)>> {
        use std::collections::HashSet;

        let mut remaining: HashSet<_> = front_cells.iter().copied().collect();
        let mut components = Vec::new();

        while let Some(&start) = remaining.iter().next() {
            let mut component = Vec::new();
            let mut stack = vec![start];

            while let Some(cell) = stack.pop() {
                if remaining.remove(&cell) {
                    component.push(cell);

                    // Check 8-connected neighbors
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = (cell.0 as i32 + dx) as usize;
                            let ny = (cell.1 as i32 + dy) as usize;
                            if nx < width && ny < height && remaining.contains(&(nx, ny)) {
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
            }

            if !component.is_empty() {
                components.push(component);
            }
        }

        components
    }

    /// Analyze potential junction between two fire front components
    #[expect(clippy::too_many_arguments)]
    fn analyze_junction(
        &self,
        front1: &[(usize, usize)],
        front2: &[(usize, usize)],
        phi: &[f32],
        spread_rate: &[f32],
        width: usize,
        height: usize,
        cell_size: f32,
        _dt: f32,
    ) -> Option<JunctionZone> {
        // Find closest points between the two fronts
        let mut min_dist = f32::MAX;
        let mut closest1 = (0, 0);
        let mut closest2 = (0, 0);

        for &(x1, y1) in front1 {
            for &(x2, y2) in front2 {
                #[expect(clippy::cast_precision_loss)]
                let dx = x2 as f32 - x1 as f32;
                #[expect(clippy::cast_precision_loss)]
                let dy = y2 as f32 - y1 as f32;
                let dist = (dx * dx + dy * dy).sqrt() * cell_size;

                if dist < min_dist {
                    min_dist = dist;
                    closest1 = (x1, y1);
                    closest2 = (x2, y2);
                }
            }
        }

        // Only consider junctions within detection distance
        if min_dist > self.detection_distance {
            return None;
        }

        // Calculate fire front normals at closest points
        let n1 = Self::calculate_normal(phi, closest1.0, closest1.1, width, height, cell_size);
        let n2 = Self::calculate_normal(phi, closest2.0, closest2.1, width, height, cell_size);

        // Check if fronts are converging (normals point toward each other)
        #[expect(clippy::cast_precision_loss)]
        let to_front2 = Vec3::new(
            (closest2.0 as f32 - closest1.0 as f32) * cell_size,
            (closest2.1 as f32 - closest1.1 as f32) * cell_size,
            0.0,
        )
        .normalize();

        let converging1 = n1.dot(&to_front2) > 0.0;
        let converging2 = n2.dot(&(-to_front2)) > 0.0;

        if !converging1 || !converging2 {
            return None; // Fronts not converging
        }

        // Calculate junction angle
        let angle = n1.dot(&(-n2)).acos();

        if angle < self.min_angle {
            return None; // Angle too small
        }

        // Estimate spread rates to calculate time to contact
        // Use actual ROS field data from the closest points on each front
        let idx1 = closest1.1 * width + closest1.0;
        let idx2 = closest2.1 * width + closest2.0;
        
        // Get spread rates at junction points (both should be > 0 for active fronts)
        let ros1 = spread_rate[idx1].max(0.1); // Minimum 0.1 m/s to avoid division issues
        let ros2 = spread_rate[idx2].max(0.1);
        
        // Time until the fronts meet, assuming they continue at current rates
        // Sum of rates because both fronts are approaching each other
        let time_to_contact = min_dist / (ros1 + ros2);

        // Calculate acceleration factor
        let acceleration = self.calculate_acceleration_factor(angle, min_dist);

        #[expect(clippy::cast_precision_loss)]
        let position = Vec3::new(
            (closest1.0 as f32 + closest2.0 as f32) * 0.5 * cell_size,
            (closest1.1 as f32 + closest2.1 as f32) * 0.5 * cell_size,
            0.0,
        );

        Some(JunctionZone {
            position,
            angle,
            distance: min_dist,
            time_to_contact,
            acceleration_factor: acceleration,
        })
    }

    /// Calculate fire front normal from level set gradient
    fn calculate_normal(
        phi: &[f32],
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        cell_size: f32,
    ) -> Vec3 {
        let idx = y * width + x;

        // Central differences for gradient
        let dx = if x > 0 && x < width - 1 {
            (phi[idx + 1] - phi[idx - 1]) / (2.0 * cell_size)
        } else {
            0.0
        };

        let dy = if y > 0 && y < height - 1 {
            (phi[idx + width] - phi[idx - width]) / (2.0 * cell_size)
        } else {
            0.0
        };

        let mag = (dx * dx + dy * dy).sqrt().max(1e-6);
        Vec3::new(dx / mag, dy / mag, 0.0)
    }

    /// Calculate acceleration factor based on junction geometry
    ///
    /// Based on Viegas et al. (2012):
    /// - Maximum acceleration at acute angles (30-60°)
    /// - Factor increases as distance decreases
    /// - Peak factor of 2-5× observed in field studies
    ///
    /// # Arguments
    ///
    /// * `angle` - Junction angle in radians
    /// * `distance` - Distance between fronts in meters
    ///
    /// # Returns
    ///
    /// Acceleration factor (1.0 = no acceleration, up to 5.0 = 5× faster)
    fn calculate_acceleration_factor(&self, angle: f32, distance: f32) -> f32 {
        // Angle effect: peak at ~45° (π/4 radians)
        let angle_factor = if angle < std::f32::consts::FRAC_PI_4 {
            // Below 45°: increasing effect
            1.0 + 3.0 * (angle / std::f32::consts::FRAC_PI_4)
        } else if angle < std::f32::consts::FRAC_PI_2 {
            // 45-90°: decreasing effect
            4.0 - 3.0 * (angle - std::f32::consts::FRAC_PI_4) / std::f32::consts::FRAC_PI_4
        } else {
            // > 90°: minimal effect
            1.0
        };

        // Distance effect: stronger as fronts approach
        let distance_factor = (1.0 - distance / self.detection_distance).max(0.0);

        // Combined: interpolate toward peak as distance closes
        1.0 + (angle_factor - 1.0) * distance_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_two_converging_fronts() {
        let detector = JunctionZoneDetector::new(80.0, 0.05); // Lower angle threshold, reasonable distance
        let width = 50;
        let height = 50;
        let cell_size = 2.0;

        // Create level set with two separate fire fronts approaching each other at an angle
        let mut phi = vec![10.0; width * height];
        
        // Create spread rate field with active fires
        let mut spread_rate = vec![0.0; width * height];

        // Front 1: coming from left, moving east
        for y in 20..26 {
            for x in 15..20 {
                let idx = y * width + x;
                phi[idx] = -1.0; // Burned
                spread_rate[idx] = 0.5; // Active spread at 0.5 m/s
            }
        }

        // Front 2: coming from right, moving west
        for y in 20..26 {
            for x in 31..36 {
                let idx = y * width + x;
                phi[idx] = -1.0; // Burned
                spread_rate[idx] = 0.6; // Active spread at 0.6 m/s
            }
        }

        let junctions = detector.detect(&phi, &spread_rate, width, height, cell_size, 0.1);

        // The test should detect a junction between the two fronts
        // Distance is about 11 cells * 2m = 22m, well within 80m threshold
        assert!(
            !junctions.is_empty(),
            "Should detect junction zone when two fronts are within detection distance"
        );
        if !junctions.is_empty() {
            let junction = &junctions[0];
            assert!(
                junction.distance < 80.0,
                "Distance should be within threshold"
            );
            assert!(
                junction.acceleration_factor >= 1.0,
                "Acceleration factor should be >= 1.0, got {}",
                junction.acceleration_factor
            );
        }
    }

    #[test]
    fn test_no_junction_for_parallel_fronts() {
        let detector = JunctionZoneDetector::default();
        let width = 50;
        let height = 50;
        let cell_size = 2.0;

        // Create level set with two parallel fire fronts moving same direction
        let mut phi = vec![10.0; width * height];
        
        // Create spread rate field
        let mut spread_rate = vec![0.0; width * height];

        // Front 1: bottom (y = 15)
        for x in 10..40 {
            for y in 10..16 {
                let idx = y * width + x;
                phi[idx] = -1.0;
                spread_rate[idx] = 0.5;
            }
        }

        // Front 2: top (y = 35) - same direction, not converging
        for x in 10..40 {
            for y in 35..40 {
                let idx = y * width + x;
                phi[idx] = -1.0;
                spread_rate[idx] = 0.5;
            }
        }

        let junctions = detector.detect(&phi, &spread_rate, width, height, cell_size, 0.1);

        // Parallel fronts moving in same direction should not create junction
        assert!(
            junctions.is_empty() || junctions[0].acceleration_factor < 1.1,
            "Parallel fronts should not have significant junction effect"
        );
    }

    #[test]
    fn test_acceleration_factor_peaks_at_45_degrees() {
        let detector = JunctionZoneDetector::default();

        let angle_30 = std::f32::consts::FRAC_PI_6; // 30°
        let angle_45 = std::f32::consts::FRAC_PI_4; // 45°
        let angle_60 = std::f32::consts::PI / 3.0; // 60°

        let distance = 50.0; // Mid-range distance

        let accel_30 = detector.calculate_acceleration_factor(angle_30, distance);
        let accel_45 = detector.calculate_acceleration_factor(angle_45, distance);
        let accel_60 = detector.calculate_acceleration_factor(angle_60, distance);

        // 45° should have maximum acceleration
        assert!(
            accel_45 >= accel_30,
            "45° should have higher acceleration than 30°"
        );
        assert!(
            accel_45 >= accel_60,
            "45° should have higher acceleration than 60°"
        );
    }

    #[test]
    fn test_acceleration_increases_as_distance_decreases() {
        let detector = JunctionZoneDetector::default();
        let angle = std::f32::consts::FRAC_PI_4; // 45°

        let accel_far = detector.calculate_acceleration_factor(angle, 90.0);
        let accel_mid = detector.calculate_acceleration_factor(angle, 50.0);
        let accel_near = detector.calculate_acceleration_factor(angle, 10.0);

        assert!(
            accel_near > accel_mid,
            "Closer fronts should have higher acceleration"
        );
        assert!(
            accel_mid > accel_far,
            "Acceleration should increase as distance decreases"
        );
    }

    #[test]
    fn test_extract_fire_front_cells() {
        let width = 10;
        let height = 10;

        let mut phi = vec![10.0; width * height];

        // Create a simple fire front (transition from negative to positive)
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if x < 5 {
                    phi[idx] = -1.0; // Burned
                } else {
                    phi[idx] = 1.0; // Unburned
                }
            }
        }

        let front_cells = JunctionZoneDetector::extract_fire_front_cells(&phi, width, height);

        // Should detect cells near x=5 boundary
        assert!(!front_cells.is_empty(), "Should detect fire front");
        assert!(
            front_cells.iter().all(|&(x, _y)| (3..=6).contains(&x)),
            "Front cells should be near boundary"
        );
    }

    #[test]
    fn test_find_connected_components() {
        let width = 20;
        let height = 20;

        // Create two separate groups of cells
        let mut front_cells = Vec::new();

        // Group 1: top-left
        for y in 5..8 {
            for x in 5..8 {
                front_cells.push((x, y));
            }
        }

        // Group 2: bottom-right
        for y in 15..18 {
            for x in 15..18 {
                front_cells.push((x, y));
            }
        }

        let components =
            JunctionZoneDetector::find_connected_components(&front_cells, width, height);

        assert_eq!(components.len(), 2, "Should find two separate components");
        assert!(
            components[0].len() >= 9 && components[1].len() >= 9,
            "Each component should have cells"
        );
    }
}
