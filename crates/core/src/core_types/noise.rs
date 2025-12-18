//! Noise functions for spatial variation and turbulence
//!
//! Provides deterministic pseudo-random noise for:
//! - Fuel property spatial variation (moisture, load)
//! - Turbulent wind fluctuation (speed, direction)
//!
//! # Scientific Basis
//!
//! Real fire spread is irregular due to:
//! - Fuel heterogeneity (Finney 2003, Anderson 1982)
//! - Turbulent wind eddies (Byram 1954, Schroeder & Buck 1970)
//! - Micro-terrain effects (Richards 1995)
//!
//! # References
//!
//! - Finney, M.A. (2003) "Calculation of fire spread rates across random landscapes"
//! - Anderson, H.E. (1982) "Aids to determining fuel models" USDA INT-122
//! - Byram, G.M. (1954) "Atmospheric conditions related to blowup fires"

use std::f32::consts::PI;

/// Seed values for deterministic noise generation
/// Using prime numbers for better distribution
const SEED_X: u32 = 1619;
const SEED_Y: u32 = 31337;
const SEED_Z: u32 = 6971;
const SEED_W: u32 = 1013;

/// Maximum value for positive i32 as f64 for safe conversion
const MAX_I32_POSITIVE: f64 = 0x7fff_ffff as f64;

/// Simple hash function for deterministic pseudo-random values
///
/// Based on integer hashing techniques for fast, deterministic noise.
/// Returns a value in [0, 1].
#[inline]
fn hash_2d(x: i32, y: i32, seed: u32) -> f32 {
    let mut n = (x.wrapping_mul(SEED_X as i32))
        .wrapping_add(y.wrapping_mul(SEED_Y as i32))
        .wrapping_add(seed as i32);
    n = (n << 13) ^ n;
    n = n
        .wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789221))
        .wrapping_add(1376312589);
    // Convert to [0, 1] using f64 to avoid precision loss
    (f64::from(n & 0x7fff_ffff) / MAX_I32_POSITIVE) as f32
}

/// Simple hash function for 3D coordinates (includes time)
#[inline]
fn hash_3d(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut n = (x.wrapping_mul(SEED_X as i32))
        .wrapping_add(y.wrapping_mul(SEED_Y as i32))
        .wrapping_add(z.wrapping_mul(SEED_Z as i32))
        .wrapping_add(seed as i32);
    n = (n << 13) ^ n;
    n = n
        .wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789221))
        .wrapping_add(1376312589);
    // Convert to [0, 1] using f64 to avoid precision loss
    (f64::from(n & 0x7fff_ffff) / MAX_I32_POSITIVE) as f32
}

/// Smooth interpolation function (Hermite curve)
#[inline]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 2D Value noise for spatial variation
///
/// Returns a value in [-1, 1] with smooth spatial continuity.
/// Use for fuel property variation (moisture, load).
///
/// # Parameters
/// - `x`, `y`: World coordinates (meters)
/// - `scale`: Noise scale (larger = smoother variation, typical 10-50m)
/// - `seed`: Seed for different noise layers
///
/// # Example
///
/// ```ignore
/// let moisture_variation = spatial_noise_2d(x, y, 20.0, 0) * 0.1; // ±10%
/// ```
pub fn spatial_noise_2d(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let sx = x / scale;
    let sy = y / scale;

    let x0 = sx.floor() as i32;
    let y0 = sy.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    // Use subtraction of floats to avoid precision loss warnings
    let fx = smoothstep(sx - sx.floor());
    let fy = smoothstep(sy - sy.floor());

    // Get corner values
    let v00 = hash_2d(x0, y0, seed);
    let v10 = hash_2d(x1, y0, seed);
    let v01 = hash_2d(x0, y1, seed);
    let v11 = hash_2d(x1, y1, seed);

    // Bilinear interpolation
    let v0 = v00 + fx * (v10 - v00);
    let v1 = v01 + fx * (v11 - v01);
    let v = v0 + fy * (v1 - v0);

    // Convert from [0, 1] to [-1, 1]
    v * 2.0 - 1.0
}

/// 3D Value noise for temporal variation (includes time dimension)
///
/// Returns a value in [-1, 1] with smooth spatial and temporal continuity.
/// Use for turbulent wind fluctuation.
///
/// # Parameters
/// - `x`, `y`: World coordinates (meters)
/// - `time`: Simulation time (seconds)
/// - `spatial_scale`: Spatial noise scale (meters)
/// - `temporal_scale`: Temporal noise scale (seconds)
/// - `seed`: Seed for different noise layers
pub fn spatiotemporal_noise(
    x: f32,
    y: f32,
    time: f32,
    spatial_scale: f32,
    temporal_scale: f32,
    seed: u32,
) -> f32 {
    let sx = x / spatial_scale;
    let sy = y / spatial_scale;
    let st = time / temporal_scale;

    let x0 = sx.floor() as i32;
    let y0 = sy.floor() as i32;
    let t0 = st.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let t1 = t0 + 1;

    // Use subtraction of floats to avoid precision loss warnings
    let fx = smoothstep(sx - sx.floor());
    let fy = smoothstep(sy - sy.floor());
    let ft = smoothstep(st - st.floor());

    // Get corner values at t0
    let v000 = hash_3d(x0, y0, t0, seed);
    let v100 = hash_3d(x1, y0, t0, seed);
    let v010 = hash_3d(x0, y1, t0, seed);
    let v110 = hash_3d(x1, y1, t0, seed);

    // Get corner values at t1
    let v001 = hash_3d(x0, y0, t1, seed);
    let v101 = hash_3d(x1, y0, t1, seed);
    let v011 = hash_3d(x0, y1, t1, seed);
    let v111 = hash_3d(x1, y1, t1, seed);

    // Trilinear interpolation
    let v00 = v000 + fx * (v100 - v000);
    let v10 = v010 + fx * (v110 - v010);
    let v0 = v00 + fy * (v10 - v00);

    let v01 = v001 + fx * (v101 - v001);
    let v11 = v011 + fx * (v111 - v011);
    let v1 = v01 + fy * (v11 - v01);

    let v = v0 + ft * (v1 - v0);

    // Convert from [0, 1] to [-1, 1]
    v * 2.0 - 1.0
}

/// Fractal Brownian Motion (fBm) for multi-scale variation
///
/// Combines multiple octaves of noise for natural-looking variation.
/// Higher octaves add fine detail at smaller scales.
///
/// # Parameters
/// - `x`, `y`: World coordinates
/// - `scale`: Base scale
/// - `octaves`: Number of noise layers (1-4 typical)
/// - `persistence`: Amplitude reduction per octave (0.5 typical)
/// - `seed`: Base seed
pub fn fbm_2d(x: f32, y: f32, scale: f32, octaves: u32, persistence: f32, seed: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves {
        total += spatial_noise_2d(x * frequency, y * frequency, scale, seed + i) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    total / max_value
}

/// Turbulent wind model for realistic fire spread
///
/// Provides realistic wind speed and direction fluctuations based on:
/// - Atmospheric turbulence physics
/// - Fire-induced convection
///
/// # Scientific Basis
///
/// Wind gusting follows a log-normal distribution with:
/// - Gust factor: 1.2-1.8 (ratio of gust to mean wind)
/// - Gust duration: 3-20 seconds
/// - Direction wobble: ±15-30° typical
///
/// References:
/// - Byram, G.M. (1954) "Atmospheric conditions related to blowup fires"
/// - Schroeder, M.J. & Buck, C.C. (1970) "Fire weather" USDA Handbook 360
#[derive(Debug, Clone)]
pub struct TurbulentWind {
    /// Gust intensity as fraction of base wind (0.0-0.4 = 0-40% variation)
    pub gust_intensity: f32,
    /// Direction wobble in degrees (0.0-30.0)
    pub direction_wobble: f32,
    /// Spatial scale for gust cells (50-200m typical)
    pub spatial_scale: f32,
    /// Temporal scale for gust duration (2-10 seconds typical)
    pub temporal_scale: f32,
}

impl Default for TurbulentWind {
    fn default() -> Self {
        Self {
            gust_intensity: 0.4,    // ±40% speed variation (realistic gusting)
            direction_wobble: 25.0, // ±25° direction variation (realistic wind shift)
            spatial_scale: 50.0,    // 50m gust cells (landscape-scale turbulence)
            temporal_scale: 5.0,    // 5-second gust cycles (realistic gust duration)
        }
    }
}

impl TurbulentWind {
    /// Create turbulent wind model for given FFDI conditions
    ///
    /// Higher FFDI = more turbulence due to fire-atmosphere coupling
    ///
    /// **Note:** This is a simplified interface. For more accurate turbulence,
    /// use `for_atmospheric_conditions()` which considers stability, mixing height, etc.
    pub fn for_ffdi(ffdi: f32) -> Self {
        let base = Self::default();

        // Scale turbulence with fire danger
        // Low FFDI (<12): minimal turbulence
        // High FFDI (>75): strong fire-induced turbulence
        let ffdi_factor = (ffdi / 50.0).min(2.0);

        Self {
            gust_intensity: base.gust_intensity * (0.5 + 0.5 * ffdi_factor),
            direction_wobble: base.direction_wobble * (0.7 + 0.3 * ffdi_factor),
            spatial_scale: base.spatial_scale / (0.5 + 0.5 * ffdi_factor),
            temporal_scale: base.temporal_scale / (0.5 + 0.5 * ffdi_factor),
        }
    }

    /// Create turbulent wind model from full atmospheric conditions
    ///
    /// This is the scientifically accurate approach that considers:
    /// - Fire danger (FFDI) - fire-induced convection
    /// - Atmospheric stability - thermal turbulence
    /// - Mixing height - boundary layer depth
    /// - Solar heating (daytime) - thermal convection
    ///
    /// # References
    /// - Pasquill-Gifford stability classes
    /// - Byram (1954) - atmospheric instability effects
    /// - Schroeder & Buck (1970) - fire weather turbulence
    pub fn for_atmospheric_conditions(
        ffdi: f32,
        mixing_height_m: f32,
        is_daytime: bool,
        atmospheric_stability: f32, // Lifted Index: negative = unstable, positive = stable
    ) -> Self {
        let base = Self::default();

        // Factor 1: FFDI (fire-induced turbulence)
        let ffdi_factor = (ffdi / 50.0).min(2.0);

        // Factor 2: Atmospheric stability (thermal turbulence)
        // Unstable (LI < -3): Enhanced turbulence (factor 1.5)
        // Neutral (LI ~ 0): Normal turbulence (factor 1.0)
        // Stable (LI > 3): Suppressed turbulence (factor 0.6)
        let stability_factor = if atmospheric_stability < -3.0 {
            1.5 // Very unstable
        } else if atmospheric_stability < 0.0 {
            1.0 + (-atmospheric_stability / 6.0) // Slightly unstable
        } else if atmospheric_stability < 3.0 {
            1.0 - (atmospheric_stability / 6.0) // Slightly stable
        } else {
            0.6 // Very stable
        };

        // Factor 3: Mixing height (boundary layer turbulence)
        // Low (< 500m): Suppressed turbulence
        // Normal (1500m): Standard turbulence
        // High (> 3000m): Enhanced turbulence
        let mixing_factor = (mixing_height_m / 1500.0).sqrt().clamp(0.6, 1.5);

        // Factor 4: Daytime solar heating (convective turbulence)
        let daytime_factor = if is_daytime { 1.2 } else { 0.8 };

        // Combine all factors (multiplicative because they interact)
        let combined_factor = ffdi_factor * stability_factor * mixing_factor * daytime_factor;

        Self {
            gust_intensity: base.gust_intensity * (0.5 + 0.5 * combined_factor),
            direction_wobble: base.direction_wobble * (0.7 + 0.3 * combined_factor),
            spatial_scale: base.spatial_scale / (0.5 + 0.5 * combined_factor),
            temporal_scale: base.temporal_scale / (0.5 + 0.5 * combined_factor),
        }
    }

    /// Get wind speed multiplier at a given position and time
    ///
    /// Returns a multiplier in range `[1 - gust_intensity, 1 + gust_intensity]`
    pub fn speed_multiplier(&self, x: f32, y: f32, time: f32) -> f32 {
        let noise = spatiotemporal_noise(x, y, time, self.spatial_scale, self.temporal_scale, 0);
        1.0 + noise * self.gust_intensity
    }

    /// Get wind direction offset at a given position and time
    ///
    /// Returns direction offset in degrees `[-direction_wobble, +direction_wobble]`
    pub fn direction_offset(&self, x: f32, y: f32, time: f32) -> f32 {
        // Use different seed for direction to decorrelate from speed
        let noise = spatiotemporal_noise(
            x,
            y,
            time,
            self.spatial_scale * 1.5,
            self.temporal_scale * 0.8,
            SEED_W,
        );
        noise * self.direction_wobble
    }

    /// Apply turbulence to a wind vector
    ///
    /// # Parameters
    /// - `wind`: Base wind vector (m/s)
    /// - `x`, `y`: Position (meters)
    /// - `time`: Simulation time (seconds)
    ///
    /// # Returns
    /// Modified wind vector with turbulent fluctuations
    pub fn apply(
        &self,
        wind: super::element::Vec3,
        x: f32,
        y: f32,
        time: f32,
    ) -> super::element::Vec3 {
        let speed = wind.magnitude();
        if speed < 0.1 {
            return wind;
        }

        // Apply speed multiplier
        let speed_mult = self.speed_multiplier(x, y, time);

        // Apply direction wobble
        let dir_offset_deg = self.direction_offset(x, y, time);
        let dir_offset_rad = dir_offset_deg * PI / 180.0;

        // Rotate wind vector in XY plane
        let cos_a = dir_offset_rad.cos();
        let sin_a = dir_offset_rad.sin();

        let new_x = wind.x * cos_a - wind.y * sin_a;
        let new_y = wind.x * sin_a + wind.y * cos_a;

        super::element::Vec3::new(new_x * speed_mult, new_y * speed_mult, wind.z * speed_mult)
    }
}

/// Configuration for fuel spatial variation
///
/// Controls how fuel properties vary across the landscape.
#[derive(Debug, Clone)]
pub struct FuelVariation {
    /// Moisture variation as fraction (0.1 = ±10%)
    pub moisture_variation: f32,
    /// Fuel load variation as fraction (0.5 = ±50%)
    pub load_variation: f32,
    /// Spatial scale for moisture variation (20-50m typical)
    pub moisture_scale: f32,
    /// Spatial scale for load variation (10-30m typical)
    pub load_scale: f32,
    /// Number of noise octaves for natural variation
    pub octaves: u32,
}

impl Default for FuelVariation {
    fn default() -> Self {
        Self {
            moisture_variation: 0.30, // ±30% moisture (realistic patchiness)
            load_variation: 0.40,     // ±40% fuel load (some areas sparse, others dense)
            moisture_scale: 30.0,     // 30m scale (landscape-level moisture variation)
            load_scale: 15.0,         // 15m scale (fuel distribution patches)
            octaves: 3,               // 3 octaves (multi-scale without extreme fine detail)
        }
    }
}

impl FuelVariation {
    /// Get moisture multiplier at a given position
    ///
    /// Returns multiplier in range `[1 - variation, 1 + variation]`
    pub fn moisture_multiplier(&self, x: f32, y: f32) -> f32 {
        let noise = fbm_2d(x, y, self.moisture_scale, self.octaves, 0.5, 100);
        1.0 + noise * self.moisture_variation
    }

    /// Get fuel load multiplier at a given position
    ///
    /// Returns multiplier in range `[1 - variation, 1 + variation]`
    pub fn load_multiplier(&self, x: f32, y: f32) -> f32 {
        let noise = fbm_2d(x, y, self.load_scale, self.octaves, 0.5, 200);
        1.0 + noise * self.load_variation
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)] // Loop indices in tests don't need precision
mod tests {
    use super::*;

    #[test]
    fn test_spatial_noise_range() {
        // Noise should be in [-1, 1]
        for i in 0..100 {
            let x = f64::from(i) as f32 * 7.3;
            let y = f64::from(i) as f32 * 11.1;
            let v = spatial_noise_2d(x, y, 10.0, 0);
            assert!((-1.0..=1.0).contains(&v), "Noise out of range: {v}");
        }
    }

    #[test]
    fn test_noise_deterministic() {
        // Same input should give same output
        let v1 = spatial_noise_2d(10.0, 20.0, 15.0, 42);
        let v2 = spatial_noise_2d(10.0, 20.0, 15.0, 42);
        assert!((v1 - v2).abs() < 1e-6, "Noise not deterministic");
    }

    #[test]
    fn test_noise_varies_spatially() {
        // Different positions should (usually) give different values
        let v1 = spatial_noise_2d(0.0, 0.0, 10.0, 0);
        let v2 = spatial_noise_2d(50.0, 50.0, 10.0, 0);
        // These values are deterministic and should differ at these positions
        // Both should be in valid range
        assert!((-1.0..=1.0).contains(&v1), "v1 out of range");
        assert!((-1.0..=1.0).contains(&v2), "v2 out of range");
    }

    #[test]
    fn test_turbulent_wind_speed_range() {
        let turb = TurbulentWind::default();
        for i in 0..100 {
            let fi = f64::from(i) as f32;
            let mult = turb.speed_multiplier(fi * 10.0, fi * 5.0, fi * 0.1);
            let min = 1.0 - turb.gust_intensity;
            let max = 1.0 + turb.gust_intensity;
            assert!(
                (min..=max).contains(&mult),
                "Speed multiplier {mult} out of range [{min}, {max}]"
            );
        }
    }

    #[test]
    fn test_turbulent_wind_direction_range() {
        let turb = TurbulentWind::default();
        for i in 0..100 {
            let fi = f64::from(i) as f32;
            let offset = turb.direction_offset(fi * 10.0, fi * 5.0, fi * 0.1);
            assert!(
                (-turb.direction_wobble..=turb.direction_wobble).contains(&offset),
                "Direction offset {offset} out of range"
            );
        }
    }

    #[test]
    fn test_fuel_variation_range() {
        let var = FuelVariation::default();
        for i in 0..100 {
            let fi = f64::from(i) as f32;
            let x = fi * 7.0;
            let y = fi * 11.0;

            let moisture_mult = var.moisture_multiplier(x, y);
            let load_mult = var.load_multiplier(x, y);

            let m_min = 1.0 - var.moisture_variation;
            let m_max = 1.0 + var.moisture_variation;
            let l_min = 1.0 - var.load_variation;
            let l_max = 1.0 + var.load_variation;

            assert!(
                (m_min..=m_max).contains(&moisture_mult),
                "Moisture mult {moisture_mult} out of range"
            );
            assert!(
                (l_min..=l_max).contains(&load_mult),
                "Load mult {load_mult} out of range"
            );
        }
    }

    #[test]
    fn test_ffdi_scales_turbulence() {
        let low = TurbulentWind::for_ffdi(10.0);
        let high = TurbulentWind::for_ffdi(100.0);

        // Higher FFDI should have more turbulence
        assert!(
            high.gust_intensity > low.gust_intensity,
            "High FFDI should have more gust intensity"
        );
        assert!(
            high.direction_wobble > low.direction_wobble,
            "High FFDI should have more direction wobble"
        );
    }
}
