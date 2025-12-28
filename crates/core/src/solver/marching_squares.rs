//! Marching squares algorithm for fire front extraction
//!
//! This module implements the marching squares algorithm to extract the fire perimeter
//! (φ = 0 contour) from the level set field.

use crate::core_types::element::Vec3;

/// Fire perimeter data for visualization and analysis
#[derive(Debug, Clone)]
pub struct FireFront {
    /// Perimeter vertices in world coordinates (x, y, z)
    pub vertices: Vec<Vec3>,
    /// Normal vectors pointing outward (toward unburned fuel)
    pub normals: Vec<Vec3>,
    /// Spread velocity at each vertex (m/s)
    pub velocities: Vec<Vec3>,
    /// Byram intensity (kW/m) at each vertex
    pub intensities: Vec<f32>,
    /// Curvature at each vertex (1/m)
    pub curvatures: Vec<f32>,
    /// Indices into vertices for multiple disconnected fronts
    pub front_starts: Vec<usize>,
}

impl FireFront {
    /// Create an empty fire front
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            velocities: Vec::new(),
            intensities: Vec::new(),
            curvatures: Vec::new(),
            front_starts: vec![0],
        }
    }

    /// Get number of vertices in the fire front
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get number of disconnected fire fronts
    pub fn front_count(&self) -> usize {
        self.front_starts.len().saturating_sub(1)
    }
}

impl Default for FireFront {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract fire front from level set field using marching squares
///
/// # Arguments
///
/// * `phi` - Level set field (φ < 0 burned, φ > 0 unburned)
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `cell_size` - Size of each cell in meters
///
/// # Returns
///
/// `FireFront` struct with vertices along the φ = 0 contour
pub fn extract_fire_front(phi: &[f32], width: u32, height: u32, cell_size: f32) -> FireFront {
    let mut front = FireFront::new();

    // Marching squares to find φ = 0 contour
    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            // Get 4 corners of cell
            let idx_tl = (y * width + x) as usize;
            let idx_tr = (y * width + x + 1) as usize;
            let idx_bl = ((y + 1) * width + x) as usize;
            let idx_br = ((y + 1) * width + x + 1) as usize;

            if idx_br >= phi.len() {
                continue;
            }

            let tl = phi[idx_tl];
            let tr = phi[idx_tr];
            let bl = phi[idx_bl];
            let br = phi[idx_br];

            // Compute marching squares case (0-15)
            let case = u8::from(tl < 0.0)
                | (u8::from(tr < 0.0) << 1)
                | (u8::from(br < 0.0) << 2)
                | (u8::from(bl < 0.0) << 3);

            // Skip empty and full cells
            if case == 0 || case == 15 {
                continue;
            }

            // Add edge vertices based on case
            // For simplicity, we'll add midpoint vertices
            // In production, use linear interpolation for smooth contours
            #[allow(clippy::cast_precision_loss)]
            let x_world = x as f32 * cell_size;
            #[allow(clippy::cast_precision_loss)]
            let y_world = y as f32 * cell_size;

            add_marching_square_vertices(
                &mut front,
                case,
                x_world,
                y_world,
                cell_size,
                (tl, tr, br, bl),
            );
        }
    }

    // Compute derived quantities
    compute_normals(&mut front, phi, width, height, cell_size);
    compute_velocities(&mut front);
    compute_intensities(&mut front);
    compute_curvatures(&mut front);

    front
}

/// Add vertices for a marching square cell
fn add_marching_square_vertices(
    front: &mut FireFront,
    case: u8,
    x: f32,
    y: f32,
    cell_size: f32,
    corners: (f32, f32, f32, f32), // (tl, tr, br, bl)
) {
    let (tl, tr, br, bl) = corners;

    // Linear interpolation factor
    let interp_top = if (tl < 0.0) == (tr < 0.0) {
        0.5
    } else {
        tl.abs() / (tl.abs() + tr.abs())
    };
    let interp_right = if (tr < 0.0) == (br < 0.0) {
        0.5
    } else {
        tr.abs() / (tr.abs() + br.abs())
    };
    let interp_bottom = if (bl < 0.0) == (br < 0.0) {
        0.5
    } else {
        bl.abs() / (bl.abs() + br.abs())
    };
    let interp_left = if (tl < 0.0) == (bl < 0.0) {
        0.5
    } else {
        tl.abs() / (tl.abs() + bl.abs())
    };

    // Edge midpoints (with interpolation)
    let top = Vec3::new(x + interp_top * cell_size, y, 0.0);
    let right = Vec3::new(x + cell_size, y + interp_right * cell_size, 0.0);
    let bottom = Vec3::new(x + interp_bottom * cell_size, y + cell_size, 0.0);
    let left = Vec3::new(x, y + interp_left * cell_size, 0.0);

    // Marching squares lookup table (simplified)
    // Each case defines which edges have contour segments
    match case {
        1 | 14 => {
            // Top-Left corner
            front.vertices.push(top);
            front.vertices.push(left);
        }
        2 | 13 => {
            // Top-Right corner
            front.vertices.push(right);
            front.vertices.push(top);
        }
        3 | 12 => {
            // Top edge
            front.vertices.push(right);
            front.vertices.push(left);
        }
        4 | 11 => {
            // Bottom-Right corner
            front.vertices.push(bottom);
            front.vertices.push(right);
        }
        5 => {
            // Two separate segments (ambiguous case)
            front.vertices.push(top);
            front.vertices.push(left);
            front.vertices.push(bottom);
            front.vertices.push(right);
        }
        6 | 9 => {
            // Right edge
            front.vertices.push(bottom);
            front.vertices.push(top);
        }
        7 | 8 => {
            // Bottom-Left corner
            front.vertices.push(left);
            front.vertices.push(bottom);
        }
        10 => {
            // Two separate segments (ambiguous case)
            front.vertices.push(top);
            front.vertices.push(right);
            front.vertices.push(left);
            front.vertices.push(bottom);
        }
        _ => {}
    }
}

/// Compute outward normal vectors from φ gradient
fn compute_normals(front: &mut FireFront, phi: &[f32], width: u32, height: u32, cell_size: f32) {
    front.normals.clear();
    front.normals.reserve(front.vertices.len());

    for vertex in &front.vertices {
        // Find nearest grid cell
        let grid_x = (vertex.x / cell_size).round() as u32;
        let grid_y = (vertex.y / cell_size).round() as u32;

        // Clamp to valid range
        let grid_x = grid_x.min(width - 1);
        let grid_y = grid_y.min(height - 1);

        // Compute gradient using central differences
        let dx = if grid_x > 0 && grid_x < width - 1 {
            let idx_right = (grid_y * width + grid_x + 1) as usize;
            let idx_left = (grid_y * width + grid_x - 1) as usize;
            if idx_right < phi.len() && idx_left < phi.len() {
                (phi[idx_right] - phi[idx_left]) / (2.0 * cell_size)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let dy = if grid_y > 0 && grid_y < height - 1 {
            let idx_up = ((grid_y + 1) * width + grid_x) as usize;
            let idx_down = ((grid_y - 1) * width + grid_x) as usize;
            if idx_up < phi.len() && idx_down < phi.len() {
                (phi[idx_up] - phi[idx_down]) / (2.0 * cell_size)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Normal = ∇φ / |∇φ| (points toward unburned)
        let mag = (dx * dx + dy * dy).sqrt();
        let normal = if mag > 1e-6 {
            Vec3::new(dx / mag, dy / mag, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0) // Default direction
        };

        front.normals.push(normal);
    }
}

/// Compute spread velocities (placeholder - will use heat flux in future)
fn compute_velocities(front: &mut FireFront) {
    front.velocities.clear();
    front.velocities.reserve(front.vertices.len());

    for normal in &front.normals {
        // Placeholder: constant spread rate of 1 m/s
        // In production, this would come from heat flux calculations
        let spread_rate = 1.0; // m/s
        front.velocities.push(*normal * spread_rate);
    }
}

/// Compute fire intensities (placeholder)
fn compute_intensities(front: &mut FireFront) {
    front.intensities.clear();
    front.intensities.resize(front.vertices.len(), 1000.0); // Placeholder: 1000 kW/m
}

/// Compute curvatures (placeholder)
fn compute_curvatures(front: &mut FireFront) {
    front.curvatures.clear();
    front.curvatures.resize(front.vertices.len(), 0.0); // Placeholder: zero curvature
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_front_creation() {
        let front = FireFront::new();
        assert_eq!(front.vertex_count(), 0);
        assert_eq!(front.front_count(), 0);
    }

    #[test]
    fn test_marching_squares_simple_circle() {
        // Create a 10x10 grid with a circular burned region in the center
        let width = 10;
        let height = 10;
        let cell_size = 1.0;
        let mut phi = vec![10.0; (width * height) as usize]; // All unburned

        // Set center region as burned (φ < 0)
        for y in 3..7 {
            for x in 3..7 {
                let idx = (y * width + x) as usize;
                phi[idx] = -5.0;
            }
        }

        let front = extract_fire_front(&phi, width, height, cell_size);

        // Should extract vertices along the perimeter
        assert!(front.vertex_count() > 0, "Fire front should have vertices");
        assert_eq!(
            front.vertices.len(),
            front.normals.len(),
            "Normals should match vertices"
        );
        assert_eq!(
            front.vertices.len(),
            front.velocities.len(),
            "Velocities should match vertices"
        );
    }

    #[test]
    fn test_empty_grid() {
        // All unburned - no fire front
        let width = 5;
        let height = 5;
        let phi = vec![10.0; (width * height) as usize];

        let front = extract_fire_front(&phi, width, height, 1.0);

        assert_eq!(
            front.vertex_count(),
            0,
            "Empty grid should have no vertices"
        );
    }

    #[test]
    fn test_fully_burned_grid() {
        // All burned - no fire front (φ = 0 is the perimeter)
        let width = 5;
        let height = 5;
        let phi = vec![-10.0; (width * height) as usize];

        let front = extract_fire_front(&phi, width, height, 1.0);

        // Fully burned interior has no perimeter in this implementation
        // (edges are clamped to ambient, so perimeter is at grid boundary)
        assert_eq!(
            front.vertex_count(),
            0,
            "Fully burned interior should have no internal front"
        );
    }
}
