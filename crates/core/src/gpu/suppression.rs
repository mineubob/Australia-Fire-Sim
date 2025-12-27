//! Suppression Integration with GPU Fire Front
//!
//! Integrates suppression coverage data with level set fire front propagation.
//! Modifies spread rate based on suppression effectiveness:
//! `R_suppressed = R_base × (1 - effectiveness)`

use crate::core_types::element::Vec3;

/// Suppression effectiveness at a specific location
#[derive(Debug, Clone, Copy)]
pub struct SuppressionEffectiveness {
    /// Coverage fraction (0-1): how much of the area is covered
    pub coverage: f32,
    /// Chemical effectiveness (0-1): how effective the suppression agent is
    pub chemical_effectiveness: f32,
    /// Overall effectiveness (0-1): combined effectiveness reducing spread rate
    pub overall_effectiveness: f32,
}

impl SuppressionEffectiveness {
    /// Create new suppression effectiveness data
    #[must_use]
    pub fn new(coverage: f32, chemical_effectiveness: f32) -> Self {
        // Overall effectiveness is product of coverage and chemical effectiveness
        // If coverage = 100% and chemical = 80%, overall = 80% reduction in spread
        let overall = coverage * chemical_effectiveness;

        Self {
            coverage: coverage.clamp(0.0, 1.0),
            chemical_effectiveness: chemical_effectiveness.clamp(0.0, 1.0),
            overall_effectiveness: overall.clamp(0.0, 1.0),
        }
    }

    /// No suppression (zero effectiveness)
    #[must_use]
    pub fn none() -> Self {
        Self {
            coverage: 0.0,
            chemical_effectiveness: 0.0,
            overall_effectiveness: 0.0,
        }
    }
}

/// Grid-based suppression coverage for GPU fire front
///
/// Stores suppression effectiveness at each grid cell for GPU computation
pub struct SuppressionGrid {
    width: u32,
    height: u32,
    grid_spacing: f32,
    /// Effectiveness value (0-1) per cell
    effectiveness: Vec<f32>,
}

impl SuppressionGrid {
    /// Create a new suppression grid
    #[must_use]
    pub fn new(width: u32, height: u32, grid_spacing: f32) -> Self {
        let cell_count = (width * height) as usize;
        Self {
            width,
            height,
            grid_spacing,
            effectiveness: vec![0.0; cell_count],
        }
    }

    /// Update suppression effectiveness at a specific grid cell
    ///
    /// # Arguments
    /// * `x` - Grid X coordinate (0 to width-1)
    /// * `y` - Grid Y coordinate (0 to height-1)
    /// * `effectiveness` - Suppression effectiveness (0-1)
    pub fn set_effectiveness(&mut self, x: u32, y: u32, effectiveness: f32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.effectiveness[idx] = effectiveness.clamp(0.0, 1.0);
        }
    }

    /// Update suppression effectiveness at a world position
    ///
    /// # Arguments
    /// * `position` - World position (meters)
    /// * `effectiveness` - Suppression effectiveness (0-1)
    /// * `radius` - Radius of effect (meters)
    pub fn set_effectiveness_at_position(
        &mut self,
        position: Vec3,
        effectiveness: f32,
        radius: f32,
    ) {
        // Convert world position to grid coordinates
        let center_x = (position.x / self.grid_spacing) as i32;
        let center_y = (position.y / self.grid_spacing) as i32;
        let radius_cells = (radius / self.grid_spacing).ceil() as i32;

        // Apply effectiveness in a circular area
        for dy in -radius_cells..=radius_cells {
            for dx in -radius_cells..=radius_cells {
                let x = center_x + dx;
                let y = center_y + dy;

                // Bounds check
                if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
                    continue;
                }

                // Calculate distance from center
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small grid cell distances to f32 - acceptable for spatial calculations"
                )]
                let dist = ((dx * dx + dy * dy) as f32).sqrt() * self.grid_spacing;

                if dist <= radius {
                    // Falloff: effectiveness decreases linearly with distance
                    let falloff = 1.0 - (dist / radius);
                    let local_effectiveness = effectiveness * falloff;

                    let idx = (y as u32 * self.width + x as u32) as usize;
                    // Use maximum effectiveness if multiple applications overlap
                    self.effectiveness[idx] = self.effectiveness[idx].max(local_effectiveness);
                }
            }
        }
    }

    /// Query suppression effectiveness at a world position
    ///
    /// Uses bilinear interpolation for smooth sampling
    #[must_use]
    pub fn query_effectiveness(&self, position: Vec3) -> SuppressionEffectiveness {
        #[expect(
            clippy::cast_precision_loss,
            reason = "Grid dimensions (u32) to f32 for position clamping - acceptable for spatial bounds"
        )]
        let grid_x = (position.x / self.grid_spacing).clamp(0.0, (self.width - 1) as f32);
        #[expect(
            clippy::cast_precision_loss,
            reason = "Grid dimensions (u32) to f32 for position clamping - acceptable for spatial bounds"
        )]
        let grid_y = (position.y / self.grid_spacing).clamp(0.0, (self.height - 1) as f32);

        // Bilinear interpolation
        let x0 = grid_x.floor() as usize;
        let y0 = grid_y.floor() as usize;
        let x1 = (x0 + 1).min((self.width - 1) as usize);
        let y1 = (y0 + 1).min((self.height - 1) as usize);

        #[expect(
            clippy::cast_precision_loss,
            reason = "Grid coordinates to f32 for interpolation - acceptable"
        )]
        let fx = grid_x - x0 as f32;
        #[expect(
            clippy::cast_precision_loss,
            reason = "Grid coordinates to f32 for interpolation - acceptable"
        )]
        let fy = grid_y - y0 as f32;

        let e00 = self.effectiveness[y0 * self.width as usize + x0];
        let e10 = self.effectiveness[y0 * self.width as usize + x1];
        let e01 = self.effectiveness[y1 * self.width as usize + x0];
        let e11 = self.effectiveness[y1 * self.width as usize + x1];

        let e0 = e00 * (1.0 - fx) + e10 * fx;
        let e1 = e01 * (1.0 - fx) + e11 * fx;
        let effectiveness = e0 * (1.0 - fy) + e1 * fy;

        // Effectiveness from grid is already overall effectiveness
        // Return it as both coverage and chemical for consistency
        SuppressionEffectiveness {
            coverage: effectiveness,
            chemical_effectiveness: 1.0,
            overall_effectiveness: effectiveness,
        }
    }

    /// Get the raw effectiveness grid for GPU upload
    #[must_use]
    pub fn effectiveness_field(&self) -> &[f32] {
        &self.effectiveness
    }

    /// Clear all suppression (set all to zero)
    pub fn clear(&mut self) {
        self.effectiveness.fill(0.0);
    }

    /// Apply evaporation/degradation over time
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds
    /// * `evaporation_rate` - Rate of effectiveness loss per second (e.g., 0.0001 = 0.01%/s)
    pub fn apply_degradation(&mut self, dt: f32, evaporation_rate: f32) {
        let decay_factor = (-evaporation_rate * dt).exp();
        for eff in &mut self.effectiveness {
            *eff *= decay_factor;
            // Remove very small values to avoid numerical issues
            if *eff < 0.001 {
                *eff = 0.0;
            }
        }
    }

    /// Get grid dimensions
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Apply suppression to spread rate field
///
/// Modifies spread rates based on suppression effectiveness:
/// `R_suppressed = R_base × (1 - effectiveness)`
///
/// # Arguments
/// * `spread_rates` - Base spread rates (m/s)
/// * `suppression_effectiveness` - Effectiveness values (0-1)
///
/// # Returns
/// Modified spread rates with suppression applied
#[must_use]
pub fn apply_suppression_to_spread_rates(
    spread_rates: &[f32],
    suppression_effectiveness: &[f32],
) -> Vec<f32> {
    assert_eq!(spread_rates.len(), suppression_effectiveness.len());

    spread_rates
        .iter()
        .zip(suppression_effectiveness.iter())
        .map(|(&rate, &eff)| {
            // R_suppressed = R × (1 - effectiveness)
            // If effectiveness = 0.8 (80%), then spread is reduced to 20%
            rate * (1.0 - eff)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppression_effectiveness() {
        let eff = SuppressionEffectiveness::new(0.8, 0.9);
        assert_eq!(eff.coverage, 0.8);
        assert_eq!(eff.chemical_effectiveness, 0.9);
        // 80% coverage × 90% chemical = 72% overall
        assert!((eff.overall_effectiveness - 0.72).abs() < 0.01);
    }

    #[test]
    fn test_suppression_grid() {
        let mut grid = SuppressionGrid::new(64, 64, 1.0);

        // Set effectiveness at grid cell (32, 32)
        grid.set_effectiveness(32, 32, 0.8);

        // Query at grid cell center - with bilinear interpolation between 4 cells
        // At (32.5, 32.5) it interpolates between cells (32,32), (33,32), (32,33), (33,33)
        // Only (32,32) has value 0.8, others are 0.0
        // Result: 0.8 * 0.25 = 0.2 (each corner contributes 25%)
        let eff = grid.query_effectiveness(Vec3::new(32.5, 32.5, 0.0));
        assert!((eff.overall_effectiveness - 0.2).abs() < 0.01);

        // Query at exact grid point (32.0, 32.0) - no interpolation
        let eff2 = grid.query_effectiveness(Vec3::new(32.0, 32.0, 0.0));
        assert!((eff2.overall_effectiveness - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_suppression_at_position() {
        let mut grid = SuppressionGrid::new(64, 64, 1.0);

        // Apply suppression in 5m radius at world position (32.0, 32.0) - exact grid point
        grid.set_effectiveness_at_position(Vec3::new(32.0, 32.0, 0.0), 0.9, 5.0);

        // Center should have high effectiveness (at center, falloff = 1.0, so eff = 0.9)
        let center = grid.query_effectiveness(Vec3::new(32.0, 32.0, 0.0));
        assert!(center.overall_effectiveness > 0.85);

        // Edge of radius should have lower effectiveness (falloff)
        // At 5m from center, falloff approaches 0
        let edge = grid.query_effectiveness(Vec3::new(37.0, 32.0, 0.0));
        assert!(edge.overall_effectiveness < center.overall_effectiveness);

        // Outside radius should have zero or very low effectiveness
        let outside = grid.query_effectiveness(Vec3::new(43.0, 32.0, 0.0));
        assert!(outside.overall_effectiveness < 0.05);
    }

    #[test]
    fn test_apply_suppression_to_spread_rates() {
        let spread_rates = vec![2.0, 3.0, 4.0];
        let suppression = vec![0.0, 0.5, 0.8];

        let result = apply_suppression_to_spread_rates(&spread_rates, &suppression);

        // No suppression: 2.0 × (1 - 0.0) = 2.0
        assert_eq!(result[0], 2.0);
        // 50% suppression: 3.0 × (1 - 0.5) = 1.5
        assert_eq!(result[1], 1.5);
        // 80% suppression: 4.0 × (1 - 0.8) = 0.8
        assert!((result[2] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_degradation() {
        let mut grid = SuppressionGrid::new(64, 64, 1.0);
        grid.set_effectiveness(32, 32, 1.0);

        // Apply degradation (1% loss per second for 10 seconds)
        grid.apply_degradation(10.0, 0.01);

        // Query at exact grid point to avoid interpolation dilution
        let eff = grid.query_effectiveness(Vec3::new(32.0, 32.0, 0.0));
        // After 10s at 1%/s: exp(-0.1) ≈ 0.905
        assert!((eff.overall_effectiveness - 0.905).abs() < 0.02);
    }
}
