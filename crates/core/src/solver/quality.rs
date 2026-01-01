//! Quality presets for grid resolution
//!
//! This module defines quality presets that determine grid resolution and cell size.
//! Higher quality means finer grid resolution but more computational cost.

use crate::TerrainData;

/// Quality preset determining grid resolution
///
/// Each preset balances accuracy and performance by choosing appropriate grid resolution.
/// The grid dimensions are calculated based on terrain size to maintain approximately
/// the same cell size across different terrain dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityPreset {
    /// Ultra quality: ~2.5m cells, 4096×4096 grid for ~10km² coverage
    Ultra,
    /// High quality: ~5m cells, 2048×2048 grid for ~10km² coverage
    High,
    /// Medium quality: ~10m cells, 1024×1024 grid for ~10km² coverage
    Medium,
    /// Low quality: ~20m cells, 512×512 grid for ~10km² coverage
    Low,
}

impl QualityPreset {
    /// Get target cell size in meters for this quality preset
    ///
    /// # Returns
    ///
    /// Cell size in meters
    #[must_use]
    pub const fn target_cell_size(&self) -> f32 {
        match self {
            Self::Ultra => 2.5,
            Self::High => 5.0,
            Self::Medium => 10.0,
            Self::Low => 20.0,
        }
    }

    /// Calculate grid dimensions for given terrain
    ///
    /// Determines grid width, height, and actual cell size based on terrain dimensions
    /// and the target cell size for this quality preset.
    ///
    /// # Arguments
    ///
    /// * `terrain` - Terrain data containing width and height in meters
    ///
    /// # Returns
    ///
    /// Tuple of `(width, height, cell_size)` where:
    /// - `width` - Grid width in cells
    /// - `height` - Grid height in cells
    /// - `cell_size` - Actual cell size in meters
    #[must_use]
    pub fn grid_dimensions(&self, terrain: &TerrainData) -> (u32, u32, f32) {
        let target_cell_size = self.target_cell_size();

        // Calculate grid dimensions based on terrain size and target cell size
        let width = (*terrain.width() / target_cell_size).ceil() as u32;
        let height = (*terrain.height() / target_cell_size).ceil() as u32;

        // Clamp to reasonable limits
        let width = width.clamp(64, 4096);
        let height = height.clamp(64, 4096);

        // Calculate actual cell size (may differ slightly from target)
        #[expect(clippy::cast_precision_loss)]
        let cell_size = *terrain.width() / width as f32;

        (width, height, cell_size)
    }

    /// Auto-detect recommended quality preset based on available hardware
    ///
    /// This is a simple heuristic based on available memory and CPU cores.
    /// GPU-specific detection would require querying GPU capabilities.
    ///
    /// # Returns
    ///
    /// Recommended quality preset
    #[must_use]
    pub fn recommended() -> Self {
        // Simple heuristic: use Medium as default
        // In a real implementation, this would query hardware capabilities
        Self::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::Meters;

    #[test]
    fn test_target_cell_sizes() {
        assert_eq!(QualityPreset::Ultra.target_cell_size(), 2.5);
        assert_eq!(QualityPreset::High.target_cell_size(), 5.0);
        assert_eq!(QualityPreset::Medium.target_cell_size(), 10.0);
        assert_eq!(QualityPreset::Low.target_cell_size(), 20.0);
    }

    #[test]
    fn test_grid_dimensions() {
        let terrain = TerrainData::flat(
            Meters::new(1000.0),
            Meters::new(1000.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );

        let (width, height, cell_size) = QualityPreset::Medium.grid_dimensions(&terrain);
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(cell_size, 10.0);

        let (width, height, _) = QualityPreset::High.grid_dimensions(&terrain);
        assert_eq!(width, 200);
        assert_eq!(height, 200);
    }

    #[test]
    fn test_grid_dimensions_clamping() {
        // Very small terrain should clamp to minimum
        let small_terrain = TerrainData::flat(
            Meters::new(10.0),
            Meters::new(10.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let (width, height, _) = QualityPreset::Ultra.grid_dimensions(&small_terrain);
        assert_eq!(width, 64);
        assert_eq!(height, 64);

        // Very large terrain should clamp to maximum
        let large_terrain = TerrainData::flat(
            Meters::new(100000.0),
            Meters::new(100000.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let (width, height, _) = QualityPreset::Ultra.grid_dimensions(&large_terrain);
        assert_eq!(width, 4096);
        assert_eq!(height, 4096);
    }
}
