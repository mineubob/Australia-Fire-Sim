use std::collections::HashMap;
use crate::core_types::element::Vec3;

/// Spatial index using hash-based octree for fast neighbor queries
pub struct SpatialIndex {
    octree: HashMap<u64, Vec<u32>>,
    cell_size: f32,
    bounds: (Vec3, Vec3),
}

impl SpatialIndex {
    /// Create a new spatial index
    pub fn new(bounds: (Vec3, Vec3), cell_size: f32) -> Self {
        SpatialIndex {
            octree: HashMap::new(),
            cell_size,
            bounds,
        }
    }
    
    /// Hash a position to a cell ID using Morton encoding
    fn hash_position(&self, pos: Vec3) -> u64 {
        let ix = ((pos.x - self.bounds.0.x) / self.cell_size).floor() as i32;
        let iy = ((pos.y - self.bounds.0.y) / self.cell_size).floor() as i32;
        let iz = ((pos.z - self.bounds.0.z) / self.cell_size).floor() as i32;
        
        morton_encode(ix, iy, iz)
    }
    
    /// Insert an element into the spatial index
    pub fn insert(&mut self, id: u32, position: Vec3) {
        let hash = self.hash_position(position);
        self.octree.entry(hash).or_insert_with(Vec::new).push(id);
    }
    
    /// Remove an element from the spatial index
    pub fn remove(&mut self, id: u32, position: Vec3) {
        let hash = self.hash_position(position);
        if let Some(cell) = self.octree.get_mut(&hash) {
            cell.retain(|&x| x != id);
            if cell.is_empty() {
                self.octree.remove(&hash);
            }
        }
    }
    
    /// Query all elements within a radius
    pub fn query_radius(&self, pos: Vec3, radius: f32) -> Vec<u32> {
        let cells_needed = (radius / self.cell_size).ceil() as i32;
        
        // OPTIMIZATION: Pre-allocate with estimated capacity
        // Most cells have ~50-100 elements, and we're checking ~27 cells (3^3)
        let estimated_capacity = ((cells_needed * 2 + 1).pow(3) as usize) * 10;
        let mut results = Vec::with_capacity(estimated_capacity.min(2000));
        
        // Check neighboring cells
        for dx in -cells_needed..=cells_needed {
            for dy in -cells_needed..=cells_needed {
                for dz in -cells_needed..=cells_needed {
                    let offset_pos = Vec3::new(
                        pos.x + dx as f32 * self.cell_size,
                        pos.y + dy as f32 * self.cell_size,
                        pos.z + dz as f32 * self.cell_size,
                    );
                    let hash = self.hash_position(offset_pos);
                    
                    if let Some(elements) = self.octree.get(&hash) {
                        results.extend(elements);
                    }
                }
            }
        }
        
        results
    }
    
    /// Clear and rebuild the entire index
    pub fn rebuild<F>(&mut self, elements: &[u32], position_fn: F)
    where
        F: Fn(u32) -> Vec3,
    {
        self.octree.clear();
        
        for &id in elements {
            let position = position_fn(id);
            self.insert(id, position);
        }
    }
    
    /// Get number of cells in the index
    pub fn cell_count(&self) -> usize {
        self.octree.len()
    }
    
    /// Get number of elements in the index
    pub fn element_count(&self) -> usize {
        self.octree.values().map(|v| v.len()).sum()
    }
}

/// Morton encode 3D coordinates into a single 64-bit integer
/// This provides spatial locality for hash lookups
fn morton_encode(x: i32, y: i32, z: i32) -> u64 {
    // Convert to unsigned to handle negative coordinates
    let x = (x as u32) as u64;
    let y = (y as u32) as u64;
    let z = (z as u32) as u64;
    
    let mut result = 0u64;
    
    for i in 0..21 {  // 21 bits per coordinate = 63 bits total
        result |= ((x & (1 << i)) << (2 * i)) |
                  ((y & (1 << i)) << (2 * i + 1)) |
                  ((z & (1 << i)) << (2 * i + 2));
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spatial_index_insert_query() {
        let bounds = (Vec3::new(-100.0, -100.0, -100.0), Vec3::new(100.0, 100.0, 100.0));
        let mut index = SpatialIndex::new(bounds, 10.0);
        
        // Insert some elements
        index.insert(1, Vec3::new(0.0, 0.0, 0.0));
        index.insert(2, Vec3::new(5.0, 5.0, 5.0));
        index.insert(3, Vec3::new(50.0, 50.0, 50.0));
        
        // Query near origin
        let nearby = index.query_radius(Vec3::new(0.0, 0.0, 0.0), 15.0);
        assert!(nearby.contains(&1));
        assert!(nearby.contains(&2));
        assert!(!nearby.contains(&3));
    }
    
    #[test]
    fn test_morton_encoding() {
        // Morton encoding should provide spatial locality
        let code1 = morton_encode(0, 0, 0);
        let code2 = morton_encode(1, 0, 0);
        let code3 = morton_encode(0, 1, 0);
        
        // Nearby points should have similar codes
        assert!(code1 != code2);
        assert!(code1 != code3);
    }
}
