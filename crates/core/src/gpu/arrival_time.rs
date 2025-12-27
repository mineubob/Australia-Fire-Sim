//! Fire Arrival Time Prediction
//!
//! Predicts when fire will reach specific locations using level set gradient tracing.
//! Uses the fire spread rate field R(x,y,t) and level set gradient to calculate
//! time-to-arrival along the optimal path.
//!
//! # Algorithm
//! 1. Start from query position
//! 2. Trace gradient of level set field φ toward fire (negative gradient direction)
//! 3. Integrate time = distance / `R_effective` along path
//! 4. Return total time when φ = 0 (fire front reached)

use crate::core_types::element::Vec3;
use crate::gpu::LevelSetSolver;

/// Result of fire arrival time prediction
#[derive(Debug, Clone, Copy)]
pub struct ArrivalPrediction {
    /// Time until fire arrival in seconds (None if fire won't reach this location)
    pub arrival_time: Option<f32>,
    /// Distance to fire front in meters
    pub distance_to_front: f32,
    /// Average spread rate along path in m/s
    pub avg_spread_rate: f32,
}

/// Predict fire arrival time at a specific position
///
/// Uses gradient tracing on the level set field to find the path fire will take
/// to reach the target position, then integrates time along that path.
///
/// # Arguments
/// * `solver` - Level set solver containing current φ field
/// * `position` - Target position (x, y) in world coordinates
/// * `spread_rates` - Current spread rate field R(x,y,t) in m/s
/// * `max_lookahead` - Maximum prediction time in seconds
///
/// # Returns
/// Prediction result with arrival time, distance, and average spread rate
///
/// # Algorithm
/// Traces the negative gradient of φ (toward fire) and integrates:
/// t = ∫ ds / R(s) along path where ds is path element
pub fn predict_arrival_time(
    solver: &LevelSetSolver,
    position: Vec3,
    spread_rates: &[f32],
    max_lookahead: f32,
) -> ArrivalPrediction {
    let (width, height) = solver.dimensions();
    let dx = solver.grid_spacing();

    // Get current phi field
    let phi = solver.read_phi();

    // Convert world position to grid coordinates
    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid dimensions (u32) to f32 for coordinate clamping - precision loss acceptable for spatial bounds"
    )]
    let grid_x = (position.x / dx).clamp(0.0, (width - 1) as f32);
    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid dimensions (u32) to f32 for coordinate clamping - precision loss acceptable for spatial bounds"
    )]
    let grid_y = (position.y / dx).clamp(0.0, (height - 1) as f32);

    // Check if already inside fire (phi < 0)
    let phi_at_pos = sample_phi(&phi, width, height, grid_x, grid_y);
    if phi_at_pos <= 0.0 {
        return ArrivalPrediction {
            arrival_time: Some(0.0),
            distance_to_front: 0.0,
            avg_spread_rate: 0.0,
        };
    }

    // Trace gradient toward fire
    let mut current_x = grid_x;
    let mut current_y = grid_y;
    let mut total_time = 0.0_f32;
    let mut total_distance = 0.0_f32;
    let step_size = 0.5; // Half grid cell for accuracy
    let max_steps = ((max_lookahead * 10.0) as usize).min(10000); // Limit iterations

    for _ in 0..max_steps {
        // Sample phi at current position
        let phi_current = sample_phi(&phi, width, height, current_x, current_y);

        // Check if reached fire front
        if phi_current <= 0.0 {
            let avg_rate = if total_time > 0.0 {
                total_distance / total_time
            } else {
                0.0
            };

            return ArrivalPrediction {
                arrival_time: Some(total_time),
                distance_to_front: total_distance,
                avg_spread_rate: avg_rate,
            };
        }

        // Calculate gradient (negative direction points toward fire)
        let grad = calculate_gradient(&phi, width, height, current_x, current_y, dx);
        let grad_mag = (grad.0 * grad.0 + grad.1 * grad.1).sqrt();

        if grad_mag < 1e-6 {
            // No gradient - can't determine path
            return ArrivalPrediction {
                arrival_time: None,
                distance_to_front: phi_current,
                avg_spread_rate: 0.0,
            };
        }

        // Move in negative gradient direction (toward fire)
        let dir_x = -grad.0 / grad_mag;
        let dir_y = -grad.1 / grad_mag;

        // Get spread rate at current position
        let spread_rate = sample_spread_rate(spread_rates, width, height, current_x, current_y);

        if spread_rate < 1e-6 {
            // No spread - fire won't reach this location
            return ArrivalPrediction {
                arrival_time: None,
                distance_to_front: phi_current,
                avg_spread_rate: 0.0,
            };
        }

        // Step along gradient
        current_x += dir_x * step_size;
        current_y += dir_y * step_size;

        // Bounds check
        #[expect(
            clippy::cast_precision_loss,
            reason = "Grid dimensions (u32) to f32 for bounds checking - precision loss acceptable for spatial comparisons"
        )]
        if current_x < 0.0
            || current_x >= (width - 1) as f32
            || current_y < 0.0
            || current_y >= (height - 1) as f32
        {
            break;
        }

        // Accumulate time and distance
        let step_distance = step_size * dx;
        let step_time = step_distance / spread_rate;
        total_distance += step_distance;
        total_time += step_time;

        // Check lookahead limit
        if total_time > max_lookahead {
            return ArrivalPrediction {
                arrival_time: None,
                distance_to_front: total_distance,
                avg_spread_rate: total_distance / total_time,
            };
        }
    }

    // Did not reach fire within max steps
    ArrivalPrediction {
        arrival_time: None,
        distance_to_front: total_distance,
        avg_spread_rate: if total_time > 0.0 {
            total_distance / total_time
        } else {
            0.0
        },
    }
}

/// Sample phi field with bilinear interpolation
fn sample_phi(phi: &[f32], width: u32, height: u32, x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min((width - 1) as i32);
    let y1 = (y0 + 1).min((height - 1) as i32);

    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid coordinates (i32) to f32 for bilinear interpolation - precision loss acceptable for interpolation"
    )]
    let fx = x - x0 as f32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid coordinates (i32) to f32 for bilinear interpolation - precision loss acceptable for interpolation"
    )]
    let fy = y - y0 as f32;

    let p00 = get_phi_safe(phi, width, height, x0, y0);
    let p10 = get_phi_safe(phi, width, height, x1, y0);
    let p01 = get_phi_safe(phi, width, height, x0, y1);
    let p11 = get_phi_safe(phi, width, height, x1, y1);

    // Bilinear interpolation
    let p0 = p00 * (1.0 - fx) + p10 * fx;
    let p1 = p01 * (1.0 - fx) + p11 * fx;
    p0 * (1.0 - fy) + p1 * fy
}

/// Get phi value with bounds checking
fn get_phi_safe(phi: &[f32], width: u32, height: u32, x: i32, y: i32) -> f32 {
    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        return 1000.0; // Large positive value (far from fire)
    }
    let idx = (y as u32 * width + x as u32) as usize;
    phi.get(idx).copied().unwrap_or(1000.0)
}

/// Calculate gradient using central differences
fn calculate_gradient(phi: &[f32], width: u32, height: u32, x: f32, y: f32, dx: f32) -> (f32, f32) {
    let ix = x as i32;
    let iy = y as i32;

    // Central differences
    let phi_xp = get_phi_safe(phi, width, height, ix + 1, iy);
    let phi_xm = get_phi_safe(phi, width, height, ix - 1, iy);
    let phi_yp = get_phi_safe(phi, width, height, ix, iy + 1);
    let phi_ym = get_phi_safe(phi, width, height, ix, iy - 1);

    let grad_x = (phi_xp - phi_xm) / (2.0 * dx);
    let grad_y = (phi_yp - phi_ym) / (2.0 * dx);

    (grad_x, grad_y)
}

/// Sample spread rate field with bilinear interpolation
fn sample_spread_rate(rates: &[f32], width: u32, height: u32, x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min((width - 1) as i32);
    let y1 = (y0 + 1).min((height - 1) as i32);

    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid coordinates (i32) to f32 for bilinear interpolation - precision loss acceptable for interpolation"
    )]
    let fx = x - x0 as f32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "Grid coordinates (i32) to f32 for bilinear interpolation - precision loss acceptable for interpolation"
    )]
    let fy = y - y0 as f32;

    let r00 = get_rate_safe(rates, width, height, x0, y0);
    let r10 = get_rate_safe(rates, width, height, x1, y0);
    let r01 = get_rate_safe(rates, width, height, x0, y1);
    let r11 = get_rate_safe(rates, width, height, x1, y1);

    // Bilinear interpolation
    let r0 = r00 * (1.0 - fx) + r10 * fx;
    let r1 = r01 * (1.0 - fx) + r11 * fx;
    r0 * (1.0 - fy) + r1 * fy
}

/// Get spread rate with bounds checking
fn get_rate_safe(rates: &[f32], width: u32, height: u32, x: i32, y: i32) -> f32 {
    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        return 0.0; // No spread outside bounds
    }
    let idx = (y as u32 * width + x as u32) as usize;
    rates.get(idx).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrival_prediction_inside_fire() {
        use crate::gpu::CpuLevelSetSolver;

        let mut solver = CpuLevelSetSolver::new(64, 64, 1.0);

        // Initialize with fire at center
        let mut phi = vec![10.0; 64 * 64];
        for j in 0..64_i32 {
            for i in 0..64_i32 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates to f32 - acceptable for test"
                )]
                let x = i as f32 - 32.0;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates to f32 - acceptable for test"
                )]
                let y = j as f32 - 32.0;
                let dist = (x * x + y * y).sqrt();
                phi[(j * 64 + i) as usize] = dist - 5.0; // Fire radius 5
            }
        }

        solver.initialize_phi(&phi);

        let spread_rates = vec![1.0; 64 * 64];
        let wrapped_solver = LevelSetSolver::Cpu(solver);

        // Test position inside fire
        let pred = predict_arrival_time(
            &wrapped_solver,
            Vec3::new(32.0, 32.0, 0.0),
            &spread_rates,
            100.0,
        );

        assert_eq!(pred.arrival_time, Some(0.0));
        assert!(pred.distance_to_front < 1.0);
    }

    #[test]
    fn test_arrival_prediction_outside_fire() {
        use crate::gpu::CpuLevelSetSolver;

        let mut solver = CpuLevelSetSolver::new(64, 64, 1.0);

        // Initialize with fire at center
        let mut phi = vec![10.0; 64 * 64];
        for j in 0..64_i32 {
            for i in 0..64_i32 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates to f32 - acceptable for test"
                )]
                let x = i as f32 - 32.0;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates to f32 - acceptable for test"
                )]
                let y = j as f32 - 32.0;
                let dist = (x * x + y * y).sqrt();
                phi[(j * 64 + i) as usize] = dist - 5.0;
            }
        }

        solver.initialize_phi(&phi);

        let spread_rates = vec![2.0; 64 * 64]; // 2 m/s uniform spread
        let wrapped_solver = LevelSetSolver::Cpu(solver);

        // Test position outside fire but reachable
        let pred = predict_arrival_time(
            &wrapped_solver,
            Vec3::new(42.0, 32.0, 0.0), // 10m from center, 5m from front
            &spread_rates,
            100.0,
        );

        // Should predict arrival (fire spreading at 2 m/s)
        assert!(pred.arrival_time.is_some());
        if let Some(time) = pred.arrival_time {
            // Should take roughly 2-3 seconds to reach (5m / 2m/s)
            assert!(time > 1.0 && time < 5.0);
        }
    }
}
