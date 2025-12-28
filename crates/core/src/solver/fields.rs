//! Field data structures for CPU and GPU backends
//!
//! This module defines the data structures used to store field values on both
//! CPU (as `Vec<f32>`) and GPU (as textures, when GPU feature is enabled).

/// Field data container for CPU backend
///
/// Stores 2D field data as a flat `Vec<f32>` in row-major order.
/// Each field represents a continuous property across the simulation grid.
#[derive(Debug, Clone)]
pub struct FieldData {
    /// Field values in row-major order (y * width + x)
    pub data: Vec<f32>,
    /// Grid width in cells
    pub width: usize,
    /// Grid height in cells
    pub height: usize,
}

impl FieldData {
    /// Create a new field with given dimensions, initialized to zero
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    ///
    /// # Returns
    ///
    /// New field initialized to all zeros
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![0.0; width * height],
            width,
            height,
        }
    }

    /// Create a new field with given dimensions, initialized to a value
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `value` - Initial value for all cells
    ///
    /// # Returns
    ///
    /// New field initialized to the specified value
    #[must_use]
    pub fn with_value(width: usize, height: usize, value: f32) -> Self {
        Self {
            data: vec![value; width * height],
            width,
            height,
        }
    }

    /// Get reference to field data
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable reference to field data
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Get value at grid position
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (0 to width-1)
    /// * `y` - Y coordinate (0 to height-1)
    ///
    /// # Returns
    ///
    /// Field value at the given position
    ///
    /// # Panics
    ///
    /// Panics if coordinates are out of bounds
    #[must_use]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        assert!(
            x < self.width && y < self.height,
            "Coordinates out of bounds"
        );
        self.data[y * self.width + x]
    }

    /// Set value at grid position
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (0 to width-1)
    /// * `y` - Y coordinate (0 to height-1)
    /// * `value` - Value to set
    ///
    /// # Panics
    ///
    /// Panics if coordinates are out of bounds
    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        assert!(
            x < self.width && y < self.height,
            "Coordinates out of bounds"
        );
        self.data[y * self.width + x] = value;
    }

    /// Fill entire field with a value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to fill with
    pub fn fill(&mut self, value: f32) {
        self.data.fill(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_creation() {
        let field = FieldData::new(10, 20);
        assert_eq!(field.width, 10);
        assert_eq!(field.height, 20);
        assert_eq!(field.data.len(), 200);
        assert!(field.data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_field_with_value() {
        let field = FieldData::with_value(5, 5, 42.0);
        assert_eq!(field.width, 5);
        assert_eq!(field.height, 5);
        assert!(field.data.iter().all(|&v| v == 42.0));
    }

    #[test]
    fn test_field_get_set() {
        let mut field = FieldData::new(10, 10);
        field.set(3, 4, 123.45);
        assert_eq!(field.get(3, 4), 123.45);

        // Verify row-major indexing
        let index = 4 * 10 + 3;
        assert_eq!(field.data[index], 123.45);
    }

    #[test]
    fn test_field_fill() {
        let mut field = FieldData::new(5, 5);
        field.fill(99.9);
        assert!(field.data.iter().all(|&v| v == 99.9));
    }

    #[test]
    #[should_panic(expected = "Coordinates out of bounds")]
    fn test_field_bounds_check() {
        let field = FieldData::new(10, 10);
        let _ = field.get(10, 5); // Out of bounds
    }
}
