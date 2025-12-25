//! Fire Front Visual Data Export
//!
//! Provides data structures and methods for exporting fire front visualization data
//! to game engines or visualization tools.
//!
//! Includes:
//! - Fire front vertices (contour extraction from level set)
//! - Velocity vectors at each vertex
//! - Byram fire intensity per segment
//! - Ready for GPU rendering

use crate::core_types::element::Vec3;

/// Fire front visual data for game engine rendering
///
/// Contains all data needed to visualize the fire front in real-time:
/// - Vertices defining the fire boundary
/// - Velocity vectors showing fire spread direction and speed
/// - Intensity values for color/size scaling
#[derive(Debug, Clone)]
pub struct FireFrontVisualData {
    /// Vertices defining the fire front contour (x, y, z coordinates)
    pub vertices: Vec<Vec3>,

    /// Velocity vectors at each vertex (m/s in x, y, z directions)
    /// Direction indicates fire spread direction, magnitude is speed
    pub velocities: Vec<Vec3>,

    /// Byram fireline intensity at each segment (kW/m)
    /// Used for visual effects (flame height, color, particle systems)
    pub intensities: Vec<f32>,

    /// Timestamp when this data was generated
    pub timestamp: f32,
}

impl FireFrontVisualData {
    /// Create new empty visual data
    #[must_use]
    pub fn new(timestamp: f32) -> Self {
        Self {
            vertices: Vec::new(),
            velocities: Vec::new(),
            intensities: Vec::new(),
            timestamp,
        }
    }

    /// Add a vertex with associated velocity and intensity
    pub fn add_vertex(&mut self, vertex: Vec3, velocity: Vec3, intensity: f32) {
        self.vertices.push(vertex);
        self.velocities.push(velocity);
        self.intensities.push(intensity);
    }

    /// Get number of vertices
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Check if data is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Calculate flame height from Byram intensity
    ///
    /// Uses Byram's formula: L = 0.0775 × I^0.46
    /// where L is flame height (m) and I is intensity (kW/m)
    ///
    /// Reference: Byram (1959)
    #[must_use]
    pub fn flame_heights(&self) -> Vec<f32> {
        self.intensities
            .iter()
            .map(|&intensity| 0.0775 * intensity.powf(0.46))
            .collect()
    }
}

/// Extract fire front contour from level set phi field using marching squares
///
/// # Arguments
/// * `phi` - Level set field (negative = inside fire, positive = outside)
/// * `width` - Grid width
/// * `height` - Grid height
/// * `grid_spacing` - Physical size of each grid cell (meters)
///
/// # Returns
/// Vector of contour vertices where φ ≈ 0 (fire boundary)
#[must_use]
pub fn extract_fire_front_contour(
    phi: &[f32],
    width: u32,
    height: u32,
    grid_spacing: f32,
) -> Vec<Vec3> {
    let mut vertices = Vec::new();

    // Marching squares algorithm - simplified implementation
    // Full implementation would use lookup tables for all 16 cases

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            let idx00 = (y * width + x) as usize;
            let idx10 = (y * width + x + 1) as usize;
            let idx01 = ((y + 1) * width + x) as usize;
            let idx11 = ((y + 1) * width + x + 1) as usize;

            let v00 = phi[idx00];
            let v10 = phi[idx10];
            let v01 = phi[idx01];
            let v11 = phi[idx11];

            // Check if cell contains zero-crossing (fire boundary)
            let has_negative = v00 < 0.0 || v10 < 0.0 || v01 < 0.0 || v11 < 0.0;
            let has_positive = v00 > 0.0 || v10 > 0.0 || v01 > 0.0 || v11 > 0.0;

            if has_negative && has_positive {
                // Simplified: add cell center as vertex
                // Full implementation would interpolate exact zero-crossing
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Grid coordinates to f32 for world position - acceptable for visualization"
                )]
                let world_x = (x as f32 + 0.5) * grid_spacing;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Grid coordinates to f32 for world position - acceptable for visualization"
                )]
                let world_y = (y as f32 + 0.5) * grid_spacing;

                vertices.push(Vec3::new(world_x, world_y, 0.0));
            }
        }
    }

    vertices
}

/// Calculate fire spread velocity at a point from level set gradient
///
/// # Arguments
/// * `phi` - Level set field
/// * `spread_rates` - Spread rate field (m/s)
/// * `x` - Grid X coordinate
/// * `y` - Grid Y coordinate
/// * `width` - Grid width
/// * `height` - Grid height
///
/// # Returns
/// Velocity vector (direction and magnitude of fire spread)
#[must_use]
pub fn calculate_fire_velocity(
    phi: &[f32],
    spread_rates: &[f32],
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Vec3 {
    // Bounds check
    if x == 0 || x >= width - 1 || y == 0 || y >= height - 1 {
        return Vec3::zeros();
    }

    let idx = (y * width + x) as usize;
    let idx_xp = (y * width + x + 1) as usize;
    let idx_xm = (y * width + x - 1) as usize;
    let idx_yp = ((y + 1) * width + x) as usize;
    let idx_ym = ((y - 1) * width + x) as usize;

    // Calculate gradient (central differences)
    let grad_x = (phi[idx_xp] - phi[idx_xm]) / 2.0;
    let grad_y = (phi[idx_yp] - phi[idx_ym]) / 2.0;

    // Gradient magnitude
    let grad_mag = (grad_x * grad_x + grad_y * grad_y).sqrt();

    if grad_mag < 1e-6 {
        return Vec3::zeros();
    }

    // Normalized gradient (direction toward fire)
    let dir_x = -grad_x / grad_mag; // Negative because fire moves from negative to positive phi
    let dir_y = -grad_y / grad_mag;

    // Spread rate at this location
    let speed = spread_rates[idx];

    // Velocity = speed × direction
    Vec3::new(dir_x * speed, dir_y * speed, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_front_visual_data() {
        let mut data = FireFrontVisualData::new(10.0);

        assert!(data.is_empty());
        assert_eq!(data.vertex_count(), 0);

        data.add_vertex(Vec3::new(10.0, 20.0, 0.0), Vec3::new(1.5, 0.5, 0.0), 1000.0);

        assert!(!data.is_empty());
        assert_eq!(data.vertex_count(), 1);
        assert_eq!(data.timestamp, 10.0);
    }

    #[test]
    fn test_flame_heights() {
        let mut data = FireFrontVisualData::new(0.0);

        // Add some vertices with different intensities
        data.add_vertex(Vec3::zeros(), Vec3::zeros(), 1000.0);
        data.add_vertex(Vec3::zeros(), Vec3::zeros(), 5000.0);
        data.add_vertex(Vec3::zeros(), Vec3::zeros(), 10000.0);

        let heights = data.flame_heights();

        assert_eq!(heights.len(), 3);
        // Byram formula: L = 0.0775 × I^0.46
        // Just check they're positive and increasing
        assert!(heights[0] > 0.0);
        assert!(heights[1] > heights[0]);
        assert!(heights[2] > heights[1]);
    }

    #[test]
    fn test_extract_fire_front_contour() {
        // Create a simple phi field with circular fire
        let width = 10_u32;
        let height = 10_u32;
        let mut phi = vec![10.0_f32; (width * height) as usize];

        // Set center cells to negative (inside fire)
        for y in 4..6 {
            for x in 4..6 {
                let idx = (y * width + x) as usize;
                phi[idx] = -5.0;
            }
        }

        let contour = extract_fire_front_contour(&phi, width, height, 1.0);

        // Should have found some boundary vertices
        assert!(!contour.is_empty());
    }

    #[test]
    fn test_calculate_fire_velocity() {
        let width = 5_u32;
        let height = 5_u32;
        let mut phi = vec![10.0_f32; (width * height) as usize];
        let spread_rates = vec![2.0_f32; (width * height) as usize];

        // Create gradient in X direction
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Test data generation - acceptable"
                )]
                let x_f32 = x as f32;
                phi[idx] = x_f32 - 2.5;
            }
        }

        // Calculate velocity at center
        let velocity = calculate_fire_velocity(&phi, &spread_rates, 2, 2, width, height);

        // Should have velocity in negative X direction (toward fire)
        assert!(velocity.x < 0.0);
        assert!(velocity.magnitude() > 0.0);
    }
}
