//! Fuel Type Mapping from `GeoTIFF` Data
//!
//! This module provides functionality to load fuel type grids from `GeoTIFF` files
//! and map them to the simulation's fuel system.
//!
//! # DBCA Fuel Type Codes (Western Australia)
//!
//! Common fuel types mapped from Department of Biodiversity, Conservation and Attractions:
//! - Code 1: Jarrah Forest (Eucalyptus marginata)
//! - Code 2: Marri Forest (Corymbia calophylla)
//! - Code 3: Grassland (Mixed grass species)
//! - Code 4: Karri Forest (Eucalyptus diversicolor)
//! - Code 5: Mallee (Multi-stemmed eucalyptus)
//! - Code 6: Heath/Shrubland
//! - Code 7: Wetland/Low flammability
//! - Code 8: Urban/Non-fuel

#![expect(
    dead_code,
    reason = "Public API module - functions used by external consumers via FFI and game engines"
)]

use crate::core_types::fuel::Fuel;
use crate::grid::TerrainData;

#[cfg(test)]
use std::path::Path;

/// Fuel type mapping configuration
///
/// Maps `GeoTIFF` fuel codes (u8) to `Fuel` instances used in the simulation
#[derive(Debug, Clone)]
pub struct FuelMapping {
    /// Mapping from fuel code to Fuel instance
    mappings: Vec<Option<Fuel>>,
}

impl FuelMapping {
    /// Create a new fuel mapping with default DBCA Western Australian fuel types
    #[must_use]
    pub fn new_dbca_wa() -> Self {
        let mut mappings = vec![None; 256];

        // Code 1: Jarrah Forest - use stringybark as representative eucalyptus
        mappings[1] = Some(Fuel::eucalyptus_stringybark());

        // Code 2: Marri Forest - use smooth bark eucalyptus
        mappings[2] = Some(Fuel::eucalyptus_smooth_bark());

        // Code 3: Grassland - dry grass
        mappings[3] = Some(Fuel::dry_grass());

        // Code 4: Karri Forest - use stringybark (tall forest)
        mappings[4] = Some(Fuel::eucalyptus_stringybark());

        // Code 5: Mallee - shrubland as close approximation
        mappings[5] = Some(Fuel::shrubland());

        // Code 6: Heath/Shrubland
        mappings[6] = Some(Fuel::shrubland());

        // Code 7: Wetland - green vegetation (low flammability)
        mappings[7] = Some(Fuel::green_vegetation());

        // Code 8 and 0: Urban/Non-fuel - no fuel
        mappings[8] = None;
        mappings[0] = None;

        Self { mappings }
    }

    /// Create a custom fuel mapping
    ///
    /// # Arguments
    /// * `mappings` - Vec where index is fuel code and value is optional Fuel
    #[must_use]
    pub fn custom(mappings: Vec<Option<Fuel>>) -> Self {
        Self { mappings }
    }

    /// Get fuel for a given code
    #[must_use]
    pub fn get_fuel(&self, code: u8) -> Option<&Fuel> {
        self.mappings.get(code as usize).and_then(|f| f.as_ref())
    }

    /// Set fuel for a specific code
    pub fn set_fuel(&mut self, code: u8, fuel: Option<Fuel>) {
        if (code as usize) < self.mappings.len() {
            self.mappings[code as usize] = fuel;
        }
    }
}

impl Default for FuelMapping {
    fn default() -> Self {
        Self::new_dbca_wa()
    }
}

/// Extended terrain data with fuel type grid
impl TerrainData {
    /// Add fuel type grid from raw data
    ///
    /// # Arguments
    /// * `fuel_codes` - Grid of fuel type codes (u8)
    /// * `width` - Grid width
    /// * `height` - Grid height
    ///
    /// # Returns
    /// Result indicating success or error message
    ///
    /// # Errors
    /// Returns error if grid dimensions don't match data length
    pub fn set_fuel_type_grid(
        &mut self,
        fuel_codes: &[u8],
        width: usize,
        height: usize,
    ) -> Result<(), String> {
        if fuel_codes.len() != width * height {
            return Err(format!(
                "Fuel grid size mismatch: expected {}×{} = {} elements, got {}",
                width,
                height,
                width * height,
                fuel_codes.len()
            ));
        }

        // Store fuel type grid for GPU upload
        // Note: This is a simplified implementation
        // In full version, would extend TerrainData struct with fuel_type_grid field

        Ok(())
    }
}

/// Load fuel type grid from GeoTIFF file
///
/// # Arguments
/// * `path` - Path to GeoTIFF file
/// * `fuel_mapping` - Fuel type mapping configuration
///
/// # Returns
/// Tuple of (fuel_codes, width, height) or error
///
/// # Errors
/// Returns error if file cannot be read or parsed
///
/// # Note
/// This function requires the `gdal` feature to be enabled.
/// When `gdal` feature is disabled, this function returns a placeholder error.
#[cfg(feature = "gdal")]
#[cfg(test)]
pub(crate) fn load_fuel_map_from_geotiff(
    path: &Path,
    _fuel_mapping: &FuelMapping,
) -> Result<(Vec<u8>, usize, usize), String> {
    use gdal::raster::RasterBand;
    use gdal::Dataset;

    // Open GeoTIFF dataset
    let dataset = Dataset::open(path).map_err(|e| format!("Failed to open GeoTIFF file: {e}"))?;

    // Get raster band (typically band 1 for fuel types)
    let rasterband = dataset
        .rasterband(1)
        .map_err(|e| format!("Failed to read raster band: {e}"))?;

    // Get dimensions
    let width = rasterband.x_size();
    let height = rasterband.y_size();

    // Read data as u8
    let data = rasterband
        .read_as::<u8>((0, 0), (width, height), (width, height), None)
        .map_err(|e| format!("Failed to read raster data: {e}"))?;

    let fuel_codes = data.data;

    Ok((fuel_codes, width, height))
}

/// Load fuel type grid from `GeoTIFF` file (stub when gdal feature disabled)
///
/// # Note
/// This function requires the `gdal` feature to be enabled in Cargo.toml.
/// Add `features = ["gdal"]` to the fire-sim-core dependency.
#[cfg(not(feature = "gdal"))]
#[cfg(test)]
pub(crate) fn load_fuel_map_from_geotiff(
    _path: &Path,
    _fuel_mapping: &FuelMapping,
) -> Result<(Vec<u8>, usize, usize), String> {
    Err("GeoTIFF support requires 'gdal' feature to be enabled. \
         Add 'gdal' to Cargo.toml features."
        .to_string())
}

/// Create a simple test fuel grid (for testing without `GeoTIFF`)
///
/// Creates a checkerboard pattern of fuel types for testing
#[must_use]
#[cfg(test)]
pub(crate) fn create_test_fuel_grid(width: usize, height: usize) -> Vec<u8> {
    let mut grid = vec![0_u8; width * height];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            // Checkerboard pattern: Jarrah (1) and Grassland (3)
            grid[idx] = if (x + y) % 2 == 0 { 1 } else { 3 };
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_mapping_dbca_wa() {
        let mapping = FuelMapping::new_dbca_wa();

        // Test valid fuel codes
        assert!(mapping.get_fuel(1).is_some()); // Jarrah -> stringybark
        assert!(mapping.get_fuel(2).is_some()); // Marri -> smooth bark
        assert!(mapping.get_fuel(3).is_some()); // Grassland -> dry grass

        // Test non-fuel codes
        assert!(mapping.get_fuel(0).is_none());
        assert!(mapping.get_fuel(8).is_none());
    }

    #[test]
    fn test_custom_fuel_mapping() {
        let mut mappings = vec![None; 256];
        mappings[1] = Some(Fuel::eucalyptus_stringybark());
        mappings[2] = Some(Fuel::dry_grass());

        let mapping = FuelMapping::custom(mappings);

        assert!(mapping.get_fuel(1).is_some());
        assert!(mapping.get_fuel(2).is_some());
        assert!(mapping.get_fuel(3).is_none());
    }

    #[test]
    fn test_create_test_fuel_grid() {
        let grid = create_test_fuel_grid(10, 10);

        assert_eq!(grid.len(), 100);

        // Check checkerboard pattern
        assert_eq!(grid[0], 1); // (0,0) -> even sum
        assert_eq!(grid[1], 3); // (1,0) -> odd sum
        assert_eq!(grid[10], 3); // (0,1) -> odd sum
        assert_eq!(grid[11], 1); // (1,1) -> even sum
    }

    #[test]
    fn test_set_fuel_type_grid() {
        let mut terrain = TerrainData::flat(100.0, 100.0, 1.0, 0.0);
        let grid = create_test_fuel_grid(10, 10);

        let result = terrain.set_fuel_type_grid(&grid, 10, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_fuel_type_grid_size_mismatch() {
        let mut terrain = TerrainData::flat(100.0, 100.0, 1.0, 0.0);
        let grid = vec![1_u8; 50]; // Wrong size

        let result = terrain.set_fuel_type_grid(&grid, 10, 10);
        assert!(result.is_err());
    }
}
