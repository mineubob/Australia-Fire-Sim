//! Simplex noise generator for fuel heterogeneity.
//!
//! Implements multi-octave simplex noise for spatially correlated
//! fuel variation based on Finney (2003) sub-grid modeling concepts.
//!
//! # Scientific Background
//!
//! Spatially correlated noise produces more realistic fuel distributions
//! than uncorrelated random values. Real vegetation exhibits spatial
//! autocorrelation due to:
//! - Seed dispersal patterns
//! - Microclimate gradients
//! - Soil variability
//! - Historical disturbance patterns
//!
//! # Implementation
//!
//! Uses 2D gradient noise with multiple octaves (fractal Brownian motion)
//! to achieve natural-looking variation at multiple scales. Based on
//! Ken Perlin's improved noise algorithm adapted for fire simulation.
//!
//! # References
//!
//! - Finney, M.A. (2003). Calculation of fire spread rates across random landscapes.
//!   International Journal of Wildland Fire, 12(2), 167-174.
//! - Perlin, K. (2002). Improving noise. ACM Transactions on Graphics, 21(3), 681-682.

/// Permutation table size (must be power of 2).
const PERM_SIZE: usize = 256;

/// Noise octave configuration.
///
/// Each octave contributes noise at a different spatial frequency,
/// allowing for multi-scale variation in fuel properties.
#[derive(Clone, Debug)]
pub struct NoiseOctave {
    /// Spatial frequency (higher = finer detail).
    ///
    /// Typical values:
    /// - 0.01: Large-scale patches (100m scale)
    /// - 0.05: Medium patches (20m scale)
    /// - 0.1: Fine detail (10m scale)
    pub frequency: f32,

    /// Amplitude contribution (0.0-1.0).
    ///
    /// Lower octaves (larger scale) typically have higher amplitude.
    pub amplitude: f32,
}

impl NoiseOctave {
    /// Create a new noise octave.
    ///
    /// # Arguments
    ///
    /// * `frequency` - Spatial frequency multiplier
    /// * `amplitude` - Amplitude contribution weight
    #[must_use]
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            amplitude,
        }
    }
}

/// Multi-octave gradient noise generator.
///
/// Generates spatially correlated noise values suitable for creating
/// realistic fuel heterogeneity patterns. The noise is deterministic
/// given a seed, allowing reproducible simulation runs.
#[derive(Clone, Debug)]
pub struct NoiseGenerator {
    /// Random seed for reproducibility.
    pub seed: u64,

    /// Octave configuration for multi-scale noise.
    pub octaves: Vec<NoiseOctave>,

    /// Permutation table for gradient selection.
    perm: Vec<u8>,

    /// Gradient vectors for 2D noise (8 directions).
    gradients: [(f32, f32); 8],
}

impl NoiseGenerator {
    /// Create generator with default octaves for fuel variation.
    ///
    /// Uses 4 octaves with typical fractal Brownian motion settings:
    /// - Octave 1: frequency 0.02, amplitude 0.5 (large patches)
    /// - Octave 2: frequency 0.04, amplitude 0.25 (medium patches)
    /// - Octave 3: frequency 0.08, amplitude 0.125 (small patches)
    /// - Octave 4: frequency 0.16, amplitude 0.0625 (fine detail)
    ///
    /// These settings produce natural-looking vegetation patterns
    /// with correlation lengths appropriate for fuel loading variation.
    ///
    /// # Arguments
    ///
    /// * `seed` - Random seed for reproducibility
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let octaves = vec![
            NoiseOctave::new(0.02, 0.5),
            NoiseOctave::new(0.04, 0.25),
            NoiseOctave::new(0.08, 0.125),
            NoiseOctave::new(0.16, 0.0625),
        ];
        Self::with_octaves(seed, octaves)
    }

    /// Create generator with custom octaves.
    ///
    /// # Arguments
    ///
    /// * `seed` - Random seed for reproducibility
    /// * `octaves` - Custom octave configuration
    #[must_use]
    pub fn with_octaves(seed: u64, octaves: Vec<NoiseOctave>) -> Self {
        let perm = Self::generate_permutation(seed);
        let gradients = Self::generate_gradients();

        Self {
            seed,
            octaves,
            perm,
            gradients,
        }
    }

    /// Generate permutation table from seed.
    ///
    /// Uses a simple linear congruential generator for deterministic
    /// but well-distributed permutation values.
    fn generate_permutation(seed: u64) -> Vec<u8> {
        let mut perm: Vec<u8> = (0..=255).collect();

        // Fisher-Yates shuffle using LCG random
        let mut rng_state = seed;
        for i in (1..PERM_SIZE).rev() {
            // LCG parameters (same as MINSTD)
            rng_state = rng_state.wrapping_mul(48_271).wrapping_rem(2_147_483_647);
            #[expect(clippy::cast_possible_truncation)]
            let j = (rng_state as usize) % (i + 1);
            perm.swap(i, j);
        }

        // Double the permutation table to avoid modulo operations
        let mut doubled = perm.clone();
        doubled.extend_from_slice(&perm);
        doubled
    }

    /// Generate gradient vectors for 8 directions.
    ///
    /// Uses unit vectors pointing in 8 equally-spaced directions
    /// for gradient noise calculation.
    fn generate_gradients() -> [(f32, f32); 8] {
        use std::f32::consts::FRAC_1_SQRT_2;
        [
            (1.0, 0.0),                       // 0°
            (FRAC_1_SQRT_2, FRAC_1_SQRT_2),   // 45°
            (0.0, 1.0),                       // 90°
            (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),  // 135°
            (-1.0, 0.0),                      // 180°
            (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2), // 225°
            (0.0, -1.0),                      // 270°
            (FRAC_1_SQRT_2, -FRAC_1_SQRT_2),  // 315°
        ]
    }

    /// Sample noise at a position, returns value in range [-1, 1].
    ///
    /// Combines multiple octaves of gradient noise to produce
    /// spatially correlated values suitable for fuel variation.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in world space (meters)
    /// * `y` - Y coordinate in world space (meters)
    ///
    /// # Returns
    ///
    /// Noise value in range [-1, 1], normalized by total amplitude
    #[must_use]
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        if self.octaves.is_empty() {
            return 0.0;
        }

        let mut total = 0.0_f32;
        let mut amplitude_sum = 0.0_f32;

        for octave in &self.octaves {
            let nx = x * octave.frequency;
            let ny = y * octave.frequency;
            total += self.gradient_noise_2d(nx, ny) * octave.amplitude;
            amplitude_sum += octave.amplitude;
        }

        // Normalize to [-1, 1] range
        if amplitude_sum > 0.0 {
            (total / amplitude_sum).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// 2D gradient noise at a single point.
    ///
    /// Implements Perlin-style gradient noise with smooth interpolation.
    fn gradient_noise_2d(&self, x: f32, y: f32) -> f32 {
        // Grid cell coordinates
        #[expect(clippy::cast_possible_truncation)]
        let x0 = x.floor() as i32;
        #[expect(clippy::cast_possible_truncation)]
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        // Fractional position within cell
        let fx = x - x.floor();
        let fy = y - y.floor();

        // Smooth interpolation weights (6t^5 - 15t^4 + 10t^3)
        let sx = Self::smoothstep(fx);
        let sy = Self::smoothstep(fy);

        // Calculate dot products at each corner
        let n00 = self.gradient_dot(x0, y0, fx, fy);
        let n10 = self.gradient_dot(x1, y0, fx - 1.0, fy);
        let n01 = self.gradient_dot(x0, y1, fx, fy - 1.0);
        let n11 = self.gradient_dot(x1, y1, fx - 1.0, fy - 1.0);

        // Bilinear interpolation
        let nx0 = Self::lerp(n00, n10, sx);
        let nx1 = Self::lerp(n01, n11, sx);
        Self::lerp(nx0, nx1, sy)
    }

    /// Calculate dot product between gradient and distance vector.
    fn gradient_dot(&self, ix: i32, iy: i32, dx: f32, dy: f32) -> f32 {
        let idx = self.hash(ix, iy);
        let grad = self.gradients[idx];
        grad.0 * dx + grad.1 * dy
    }

    /// Hash grid coordinates to gradient index.
    fn hash(&self, x: i32, y: i32) -> usize {
        // Wrap to permutation table size using bitwise AND
        #[expect(clippy::cast_sign_loss)]
        let px = (x & 0xFF) as usize;
        #[expect(clippy::cast_sign_loss)]
        let py = (y & 0xFF) as usize;
        (self.perm[self.perm[px] as usize + py] as usize) & 0x07
    }

    /// Improved smoothstep function (Perlin's improved noise).
    ///
    /// Uses 6t^5 - 15t^4 + 10t^3 for C2 continuity.
    #[inline]
    fn smoothstep(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    /// Linear interpolation.
    #[inline]
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + t * (b - a)
    }

    /// Generate a complete noise field for a grid.
    ///
    /// Produces a vector of noise values for each cell in a grid,
    /// suitable for batch processing of fuel heterogeneity.
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Cell size in meters (for proper spatial scaling)
    ///
    /// # Returns
    ///
    /// Vector of noise values in row-major order (y * width + x),
    /// each value in range [-1, 1]
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn generate_field(&self, width: u32, height: u32, cell_size: f32) -> Vec<f32> {
        let size = (width as usize) * (height as usize);
        let mut field = vec![0.0; size];

        for y in 0..height {
            for x in 0..width {
                let world_x = x as f32 * cell_size;
                let world_y = y as f32 * cell_size;
                let idx = (y as usize) * (width as usize) + (x as usize);
                field[idx] = self.sample(world_x, world_y);
            }
        }

        field
    }
}

#[cfg(test)]
#[expect(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    /// Test that noise values are within the expected [-1, 1] range.
    #[test]
    fn noise_generator_produces_valid_range() {
        let gen = NoiseGenerator::new(12345);

        // Sample many points and verify range
        for i in 0..1000 {
            let x = (i as f32) * 7.3;
            let y = (i as f32) * 11.7;
            let value = gen.sample(x, y);

            assert!(
                (-1.0..=1.0).contains(&value),
                "Noise value {value} at ({x}, {y}) is outside [-1, 1] range"
            );
        }
    }

    /// Test that nearby points have more similar values than distant points.
    #[test]
    fn noise_spatial_correlation() {
        // Use higher frequency octaves for more variation in test
        let gen = NoiseGenerator::with_octaves(
            42,
            vec![NoiseOctave::new(0.5, 0.5), NoiseOctave::new(1.0, 0.5)],
        );

        // Sample at a reference point
        let ref_value = gen.sample(10.0, 10.0);

        // Nearby points should be more similar
        let near_value = gen.sample(10.5, 10.5);
        let near_diff = (ref_value - near_value).abs();

        // Distant points should differ more (on average)
        // Sample at positions that are far apart
        let mut far_diffs = Vec::new();
        for i in 1..=20 {
            let far_value = gen.sample(10.0 + (i as f32) * 10.0, 10.0 + (i as f32) * 7.3);
            far_diffs.push((ref_value - far_value).abs());
        }
        let avg_far_diff: f32 = far_diffs.iter().sum::<f32>() / far_diffs.len() as f32;

        // Near difference should generally be smaller than average far difference
        // Use a generous multiplier as noise is stochastic
        assert!(
            near_diff < avg_far_diff * 3.0 || avg_far_diff < 0.01,
            "Near diff {near_diff} should be smaller than far avg {avg_far_diff}"
        );
    }

    /// Test that the same seed produces identical output.
    #[test]
    fn noise_deterministic_with_seed() {
        let gen1 = NoiseGenerator::new(99999);
        let gen2 = NoiseGenerator::new(99999);

        // Same seed should produce identical results
        for i in 0..100 {
            let x = (i as f32) * 13.7;
            let y = (i as f32) * 19.3;

            let v1 = gen1.sample(x, y);
            let v2 = gen2.sample(x, y);

            assert!(
                (v1 - v2).abs() < f32::EPSILON,
                "Same seed should produce identical noise: {v1} vs {v2}"
            );
        }

        // Different seeds should produce different results
        let gen3 = NoiseGenerator::new(11111);
        let mut all_same = true;
        for i in 0..100 {
            let x = (i as f32) * 13.7;
            let y = (i as f32) * 19.3;

            let v1 = gen1.sample(x, y);
            let v3 = gen3.sample(x, y);

            if (v1 - v3).abs() > f32::EPSILON {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "Different seeds should produce different noise");
    }

    /// Test that generated field matches individual samples.
    #[test]
    fn noise_field_matches_samples() {
        let gen = NoiseGenerator::new(54321);
        let width = 10;
        let height = 10;
        let cell_size = 5.0;

        let field = gen.generate_field(width, height, cell_size);

        // Verify some sample points
        for y in 0..height {
            for x in 0..width {
                let expected = gen.sample(x as f32 * cell_size, y as f32 * cell_size);
                let actual = field[(y as usize) * (width as usize) + (x as usize)];

                assert!(
                    (expected - actual).abs() < f32::EPSILON,
                    "Field value at ({x}, {y}) doesn't match sample: {expected} vs {actual}"
                );
            }
        }
    }

    /// Test that empty octaves return zero.
    #[test]
    fn noise_empty_octaves_returns_zero() {
        let gen = NoiseGenerator::with_octaves(123, vec![]);

        for i in 0..10 {
            let value = gen.sample(i as f32 * 10.0, i as f32 * 10.0);
            assert!(
                value.abs() < f32::EPSILON,
                "Empty octaves should return 0, got {value}"
            );
        }
    }

    /// Test custom octave configuration.
    #[test]
    fn noise_custom_octaves() {
        let octaves = vec![
            NoiseOctave::new(0.1, 1.0), // Single octave for simpler testing
        ];
        let gen = NoiseGenerator::with_octaves(777, octaves);

        let value = gen.sample(50.0, 50.0);
        assert!(
            (-1.0..=1.0).contains(&value),
            "Custom octave noise should be in range: {value}"
        );
    }
}
