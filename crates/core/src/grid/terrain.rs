//! Terrain elevation and topography support for realistic fire simulation
//!
//! Implements Digital Elevation Model (DEM) support with slope/aspect calculations,
//! solar radiation based on terrain, and efficient height queries.

use crate::core_types::element::Vec3;
use serde::{Deserialize, Serialize};

/// Precomputed terrain properties cache for performance
/// Stores slope and aspect at each grid position to avoid runtime computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainCache {
    /// Precomputed slope at each XY position (degrees)
    pub slope: Vec<f32>,
    /// Precomputed aspect at each XY position (degrees, 0-360)
    pub aspect: Vec<f32>,
    /// Number of samples in X direction
    pub nx: usize,
    /// Number of samples in Y direction
    pub ny: usize,
}

impl TerrainCache {
    /// Get cached slope at grid position
    #[inline]
    pub fn slope_at_grid(&self, ix: usize, iy: usize) -> f32 {
        self.slope[iy * self.nx + ix]
    }

    /// Get cached aspect at grid position
    #[inline]
    pub fn aspect_at_grid(&self, ix: usize, iy: usize) -> f32 {
        self.aspect[iy * self.nx + ix]
    }
}

/// Terrain data structure holding elevation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainData {
    /// Width of terrain in meters
    pub(crate) width: f32,
    /// Height of terrain in meters
    pub(crate) height: f32,
    /// Grid resolution in meters per sample
    pub(crate) resolution: f32,
    /// Number of elevation samples in X direction
    pub(crate) nx: usize,
    /// Number of elevation samples in Y direction
    pub(crate) ny: usize,
    /// Elevation data in meters (row-major order: [y * nx + x])
    pub(crate) elevations: Vec<f32>,
    /// Minimum elevation in dataset
    pub(crate) min_elevation: f32,
    /// Maximum elevation in dataset
    pub(crate) max_elevation: f32,
}

impl TerrainData {
    /// Create flat terrain at given elevation
    pub fn flat(width: f32, height: f32, resolution: f32, elevation: f32) -> Self {
        let nx = (width / resolution).ceil() as usize + 1;
        let ny = (height / resolution).ceil() as usize + 1;
        let elevations = vec![elevation; nx * ny];

        TerrainData {
            width,
            height,
            resolution,
            nx,
            ny,
            elevations,
            min_elevation: elevation,
            max_elevation: elevation,
        }
    }

    /// Create terrain with a single hill
    pub fn single_hill(
        width: f32,
        height: f32,
        resolution: f32,
        base_elevation: f32,
        hill_height: f32,
        hill_radius: f32,
    ) -> Self {
        let nx = (width / resolution).ceil() as usize + 1;
        let ny = (height / resolution).ceil() as usize + 1;
        let mut elevations = Vec::with_capacity(nx * ny);

        let center_x = width / 2.0;
        let center_y = height / 2.0;

        let mut min_elev = f32::MAX;
        let mut max_elev = f32::MIN;

        for iy in 0..ny {
            for ix in 0..nx {
                let x = ix as f32 * resolution;
                let y = iy as f32 * resolution;

                let dx = x - center_x;
                let dy = y - center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                // Gaussian hill profile
                let height_factor = (-dist * dist / (hill_radius * hill_radius)).exp();
                let elev = base_elevation + hill_height * height_factor;

                elevations.push(elev);
                min_elev = min_elev.min(elev);
                max_elev = max_elev.max(elev);
            }
        }

        TerrainData {
            width,
            height,
            resolution,
            nx,
            ny,
            elevations,
            min_elevation: min_elev,
            max_elevation: max_elev,
        }
    }

    /// Create terrain with valley between two hills
    pub fn valley_between_hills(
        width: f32,
        height: f32,
        resolution: f32,
        base_elevation: f32,
        hill_height: f32,
    ) -> Self {
        let nx = (width / resolution).ceil() as usize + 1;
        let ny = (height / resolution).ceil() as usize + 1;
        let mut elevations = Vec::with_capacity(nx * ny);

        let hill1_x = width * 0.25;
        let hill2_x = width * 0.75;
        let center_y = height / 2.0;
        let hill_radius = width * 0.2;

        let mut min_elev = f32::MAX;
        let mut max_elev = f32::MIN;

        for iy in 0..ny {
            for ix in 0..nx {
                let x = ix as f32 * resolution;
                let y = iy as f32 * resolution;

                // Distance to first hill
                let dx1 = x - hill1_x;
                let dy1 = y - center_y;
                let dist1 = (dx1 * dx1 + dy1 * dy1).sqrt();
                let height1 = hill_height * (-dist1 * dist1 / (hill_radius * hill_radius)).exp();

                // Distance to second hill
                let dx2 = x - hill2_x;
                let dy2 = y - center_y;
                let dist2 = (dx2 * dx2 + dy2 * dy2).sqrt();
                let height2 = hill_height * (-dist2 * dist2 / (hill_radius * hill_radius)).exp();

                // Valley effect (negative between hills)
                let valley_x = (x - width / 2.0) / (width * 0.25);
                let valley_depth = -10.0 * (-(valley_x * valley_x)).exp();

                let elev = base_elevation + height1 + height2 + valley_depth;

                elevations.push(elev);
                min_elev = min_elev.min(elev);
                max_elev = max_elev.max(elev);
            }
        }

        TerrainData {
            width,
            height,
            resolution,
            nx,
            ny,
            elevations,
            min_elevation: min_elev,
            max_elevation: max_elev,
        }
    }

    /// Create terrain from a heightmap array
    ///
    /// # Arguments
    /// * `width` - Width of terrain in meters
    /// * `height` - Height of terrain in meters
    /// * `heightmap` - 2D heightmap as 1D array in row-major order [y * nx + x]
    /// * `nx` - Number of samples in X direction
    /// * `ny` - Number of samples in Y direction
    /// * `elevation_scale` - Multiplier for heightmap values (heightmap values are [0,1])
    /// * `base_elevation` - Base elevation to add to all heights
    pub fn from_heightmap(
        width: f32,
        height: f32,
        heightmap: Vec<f32>,
        nx: usize,
        ny: usize,
        elevation_scale: f32,
        base_elevation: f32,
    ) -> Self {
        assert_eq!(heightmap.len(), nx * ny, "Heightmap size mismatch");

        let resolution = width / (nx - 1) as f32;

        let mut min_elev = f32::MAX;
        let mut max_elev = f32::MIN;

        let elevations: Vec<f32> = heightmap
            .iter()
            .map(|&h| {
                let elev = base_elevation + h * elevation_scale;
                min_elev = min_elev.min(elev);
                max_elev = max_elev.max(elev);
                elev
            })
            .collect();

        TerrainData {
            width,
            height,
            resolution,
            nx,
            ny,
            elevations,
            min_elevation: min_elev,
            max_elevation: max_elev,
        }
    }

    /// Query elevation at world position (x, y) using bilinear interpolation
    pub fn elevation_at(&self, x: f32, y: f32) -> f32 {
        // Clamp to terrain bounds
        let x_clamped = x.max(0.0).min(self.width);
        let y_clamped = y.max(0.0).min(self.height);

        // Convert to grid coordinates
        let gx = x_clamped / self.resolution;
        let gy = y_clamped / self.resolution;

        // Get integer grid cell
        let ix0 = (gx.floor() as usize).min(self.nx - 2);
        let iy0 = (gy.floor() as usize).min(self.ny - 2);
        let ix1 = ix0 + 1;
        let iy1 = iy0 + 1;

        // Fractional parts for interpolation
        let fx = gx - ix0 as f32;
        let fy = gy - iy0 as f32;

        // Get four corner elevations
        let e00 = self.elevations[iy0 * self.nx + ix0];
        let e10 = self.elevations[iy0 * self.nx + ix1];
        let e01 = self.elevations[iy1 * self.nx + ix0];
        let e11 = self.elevations[iy1 * self.nx + ix1];

        // Bilinear interpolation
        let e0 = e00 * (1.0 - fx) + e10 * fx;
        let e1 = e01 * (1.0 - fx) + e11 * fx;
        e0 * (1.0 - fy) + e1 * fy
    }

    /// Calculate slope angle at position in degrees using simple 4-point gradient
    ///
    /// This is a fast approximation suitable for most use cases.
    /// For more accurate results, use `slope_at_horn()` which uses Horn's method.
    pub fn slope_at(&self, x: f32, y: f32) -> f32 {
        let delta = self.resolution;

        // Sample elevations around point
        let z_east = self.elevation_at(x + delta, y);
        let z_west = self.elevation_at(x - delta, y);
        let z_north = self.elevation_at(x, y + delta);
        let z_south = self.elevation_at(x, y - delta);

        // Calculate gradients
        let dz_dx = (z_east - z_west) / (2.0 * delta);
        let dz_dy = (z_north - z_south) / (2.0 * delta);

        // Slope magnitude
        let slope_rad = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt().atan();
        slope_rad.to_degrees()
    }

    /// Calculate slope angle using Horn's method (3x3 kernel)
    ///
    /// Horn's method provides more accurate slope estimation by using
    /// a 3x3 neighborhood kernel that weighs diagonal neighbors less.
    ///
    /// # Scientific Reference
    /// Horn, B.K.P. (1981). "Hill Shading and the Reflectance Map."
    /// Proceedings of the IEEE, 69(1), 14-47.
    ///
    /// # Returns
    /// Slope angle in degrees (0° = flat, 90° = vertical)
    pub fn slope_at_horn(&self, x: f32, y: f32) -> f32 {
        let d = self.resolution;

        // Sample 3x3 neighborhood
        // z[0] z[1] z[2]   (NW) (N) (NE)
        // z[3] z[4] z[5]   (W)  (C) (E)
        // z[6] z[7] z[8]   (SW) (S) (SE)
        let z = [
            self.elevation_at(x - d, y + d), // NW (0)
            self.elevation_at(x, y + d),     // N  (1)
            self.elevation_at(x + d, y + d), // NE (2)
            self.elevation_at(x - d, y),     // W  (3)
            self.elevation_at(x, y),         // C  (4)
            self.elevation_at(x + d, y),     // E  (5)
            self.elevation_at(x - d, y - d), // SW (6)
            self.elevation_at(x, y - d),     // S  (7)
            self.elevation_at(x + d, y - d), // SE (8)
        ];

        // Horn's method gradient calculation
        // dz/dx = ((z[2] + 2*z[5] + z[8]) - (z[0] + 2*z[3] + z[6])) / (8 * d)
        // dz/dy = ((z[6] + 2*z[7] + z[8]) - (z[0] + 2*z[1] + z[2])) / (8 * d)
        let dz_dx = ((z[2] + 2.0 * z[5] + z[8]) - (z[0] + 2.0 * z[3] + z[6])) / (8.0 * d);
        let dz_dy = ((z[6] + 2.0 * z[7] + z[8]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * d);

        // Slope magnitude
        let slope_rad = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt().atan();
        slope_rad.to_degrees()
    }

    /// Calculate aspect using Horn's method (3x3 kernel)
    ///
    /// # Scientific Reference
    /// Horn, B.K.P. (1981). "Hill Shading and the Reflectance Map."
    ///
    /// # Returns
    /// Aspect in degrees (0° = North, 90° = East, 180° = South, 270° = West)
    pub fn aspect_at_horn(&self, x: f32, y: f32) -> f32 {
        let d = self.resolution;

        // Sample 3x3 neighborhood
        let z = [
            self.elevation_at(x - d, y + d), // NW (0)
            self.elevation_at(x, y + d),     // N  (1)
            self.elevation_at(x + d, y + d), // NE (2)
            self.elevation_at(x - d, y),     // W  (3)
            self.elevation_at(x, y),         // C  (4)
            self.elevation_at(x + d, y),     // E  (5)
            self.elevation_at(x - d, y - d), // SW (6)
            self.elevation_at(x, y - d),     // S  (7)
            self.elevation_at(x + d, y - d), // SE (8)
        ];

        // Horn's method gradient calculation
        let dz_dx = ((z[2] + 2.0 * z[5] + z[8]) - (z[0] + 2.0 * z[3] + z[6])) / (8.0 * d);
        let dz_dy = ((z[6] + 2.0 * z[7] + z[8]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * d);

        // Aspect is direction of steepest descent
        // Using atan2 to get direction in -180 to 180 range
        let aspect_rad = (-dz_dx).atan2(-dz_dy);
        let aspect_deg = aspect_rad.to_degrees();

        // Convert to 0-360 range with N=0
        if aspect_deg < 0.0 {
            aspect_deg + 360.0
        } else {
            aspect_deg
        }
    }

    /// Calculate aspect (direction of slope) at position in degrees (0-360)
    /// 0° = North, 90° = East, 180° = South, 270° = West
    pub fn aspect_at(&self, x: f32, y: f32) -> f32 {
        let delta = self.resolution;

        let z_east = self.elevation_at(x + delta, y);
        let z_west = self.elevation_at(x - delta, y);
        let z_north = self.elevation_at(x, y + delta);
        let z_south = self.elevation_at(x, y - delta);

        let dz_dx = (z_east - z_west) / (2.0 * delta);
        let dz_dy = (z_north - z_south) / (2.0 * delta);

        // Aspect is perpendicular to gradient direction
        // atan2(dz_dx, dz_dy) gives direction of steepest ascent
        let aspect_rad = dz_dx.atan2(dz_dy);
        let aspect_deg = aspect_rad.to_degrees();

        // Convert to 0-360 range
        if aspect_deg < 0.0 {
            aspect_deg + 360.0
        } else {
            aspect_deg
        }
    }

    /// Calculate solar radiation modifier based on terrain (0-1 scale)
    /// Accounts for slope and aspect relative to sun position
    pub fn solar_radiation_factor(
        &self,
        x: f32,
        y: f32,
        sun_azimuth: f32,
        sun_elevation: f32,
    ) -> f32 {
        let slope = self.slope_at(x, y).to_radians();
        let aspect = self.aspect_at(x, y).to_radians();
        let sun_az = sun_azimuth.to_radians();
        let sun_el = sun_elevation.to_radians();

        // Calculate angle between surface normal and sun direction
        // Surface normal based on slope and aspect
        let nx = slope.sin() * aspect.sin();
        let ny = slope.sin() * aspect.cos();
        let nz = slope.cos();

        // Sun direction vector
        let sx = sun_el.cos() * sun_az.sin();
        let sy = sun_el.cos() * sun_az.cos();
        let sz = sun_el.sin();

        // Dot product gives cosine of angle
        let cos_angle = nx * sx + ny * sy + nz * sz;

        // Radiation proportional to cosine (Lambert's law)
        cos_angle.max(0.0)
    }

    /// Build terrain cache for fast slope/aspect lookups
    /// Precomputes slope and aspect for every grid position
    /// This is expensive but only done once at initialization
    pub fn build_cache(&self, cache_nx: usize, cache_ny: usize, cell_size: f32) -> TerrainCache {
        let mut slope = Vec::with_capacity(cache_nx * cache_ny);
        let mut aspect = Vec::with_capacity(cache_nx * cache_ny);

        for iy in 0..cache_ny {
            for ix in 0..cache_nx {
                let x = ix as f32 * cell_size + cell_size / 2.0;
                let y = iy as f32 * cell_size + cell_size / 2.0;

                slope.push(self.slope_at(x, y));
                aspect.push(self.aspect_at(x, y));
            }
        }

        TerrainCache {
            slope,
            aspect,
            nx: cache_nx,
            ny: cache_ny,
        }
    }

    /// Get gradient vector at position (dz/dx, dz/dy, 1.0 normalized)
    pub fn gradient_at(&self, x: f32, y: f32) -> Vec3 {
        let delta = self.resolution;

        let z_east = self.elevation_at(x + delta, y);
        let z_west = self.elevation_at(x - delta, y);
        let z_north = self.elevation_at(x, y + delta);
        let z_south = self.elevation_at(x, y - delta);

        let dz_dx = (z_east - z_west) / (2.0 * delta);
        let dz_dy = (z_north - z_south) / (2.0 * delta);

        Vec3::new(dz_dx, dz_dy, 1.0).normalize()
    }

    /// Get terrain width in meters
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get terrain height in meters
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Get minimum elevation in meters
    pub fn min_elevation(&self) -> f32 {
        self.min_elevation
    }

    /// Get maximum elevation in meters
    pub fn max_elevation(&self) -> f32 {
        self.max_elevation
    }

    /// Get terrain resolution in meters
    pub fn resolution(&self) -> f32 {
        self.resolution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_flat_terrain() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 50.0);

        assert_eq!(terrain.elevation_at(25.0, 25.0), 50.0);
        assert_eq!(terrain.elevation_at(75.0, 75.0), 50.0);
        assert_relative_eq!(terrain.slope_at(50.0, 50.0), 0.0, epsilon = 0.1);
    }

    #[test]
    fn test_single_hill() {
        let terrain = TerrainData::single_hill(200.0, 200.0, 5.0, 50.0, 100.0, 50.0);

        // Peak should be at center
        let peak_elevation = terrain.elevation_at(100.0, 100.0);
        assert!(peak_elevation > 140.0);
        // Allow for realistic Gaussian peak
        assert!(peak_elevation <= 150.0);

        // Edges should be close to base
        let edge_elevation = terrain.elevation_at(10.0, 10.0);
        assert!(edge_elevation < 60.0);

        // Slope at peak should be near zero
        let slope_at_peak = terrain.slope_at(100.0, 100.0);
        assert!(slope_at_peak < 1.0);

        // Slope on hillside should be significant
        let slope_on_side = terrain.slope_at(100.0, 130.0);
        assert!(slope_on_side > 5.0);
    }

    #[test]
    fn test_valley() {
        let terrain = TerrainData::valley_between_hills(400.0, 200.0, 5.0, 50.0, 80.0);

        // Hills should be higher than base
        let hill1 = terrain.elevation_at(100.0, 100.0);
        let hill2 = terrain.elevation_at(300.0, 100.0);
        assert!(hill1 > 80.0);
        assert!(hill2 > 80.0);

        // Valley center should be lower
        let valley = terrain.elevation_at(200.0, 100.0);
        assert!(valley < hill1);
        assert!(valley < hill2);
    }

    #[test]
    fn test_interpolation() {
        let terrain = TerrainData::single_hill(100.0, 100.0, 10.0, 0.0, 100.0, 30.0);

        // Query between grid points
        let e1 = terrain.elevation_at(45.0, 45.0);
        let e2 = terrain.elevation_at(50.0, 50.0);
        let e3 = terrain.elevation_at(55.0, 55.0);

        // Should be smooth (no sudden jumps)
        assert!((e2 - e1).abs() < 20.0);
        assert!((e3 - e2).abs() < 20.0);
    }

    #[test]
    fn test_solar_radiation() {
        let terrain = TerrainData::single_hill(100.0, 100.0, 5.0, 0.0, 50.0, 30.0);

        // Flat horizontal surface with sun directly overhead
        let flat_factor = terrain.solar_radiation_factor(10.0, 10.0, 0.0, 90.0);
        assert!(flat_factor > 0.9); // Should be close to 1.0

        // Test with sun angle - results vary by position and slope
        let some_slope = terrain.solar_radiation_factor(50.0, 30.0, 180.0, 45.0);
        assert!(some_slope >= 0.0); // Just verify it's computed without error
        assert!(some_slope <= 1.0);
    }

    #[test]
    fn test_from_heightmap() {
        // Create a simple 3x3 heightmap
        let heightmap = vec![
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, // Peak in center
            0.0, 0.0, 0.0,
        ];

        let terrain = TerrainData::from_heightmap(
            100.0, 100.0, heightmap, 3, 3, 50.0, // Scale: 1.0 in heightmap = 50m elevation
            10.0, // Base elevation
        );

        assert_eq!(terrain.width, 100.0);
        assert_eq!(terrain.height, 100.0);
        assert_eq!(terrain.nx, 3);
        assert_eq!(terrain.ny, 3);
        assert_eq!(terrain.min_elevation, 10.0);
        assert_eq!(terrain.max_elevation, 60.0); // 10 + 1.0 * 50

        // Check center is highest
        let center_elev = terrain.elevation_at(50.0, 50.0);
        assert!(center_elev > 55.0); // Should be close to 60
    }
}
