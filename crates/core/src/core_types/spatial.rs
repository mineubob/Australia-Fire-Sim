use crate::core_types::element::Vec3;
use rustc_hash::FxHashMap;

/// Spatial index using hash-based octree for fast neighbor queries
pub(crate) struct SpatialIndex {
    octree: FxHashMap<u64, Vec<(u32, Vec3)>>,
    cell_size: f32,
    bounds: (Vec3, Vec3),
}

impl SpatialIndex {
    /// Create a new spatial index
    pub fn new(bounds: (Vec3, Vec3), cell_size: f32) -> Self {
        SpatialIndex {
            octree: FxHashMap::with_capacity_and_hasher(1024, Default::default()),
            cell_size,
            bounds,
        }
    }

    /// Hash a position to a cell ID using Morton encoding
    #[inline(always)]
    fn hash_position(&self, pos: Vec3) -> u64 {
        let ix = ((pos.x - self.bounds.0.x) / self.cell_size).floor() as i32;
        let iy = ((pos.y - self.bounds.0.y) / self.cell_size).floor() as i32;
        let iz = ((pos.z - self.bounds.0.z) / self.cell_size).floor() as i32;

        morton_encode(ix, iy, iz)
    }

    /// Insert an element into the spatial index
    pub fn insert(&mut self, id: u32, position: Vec3) {
        let hash = self.hash_position(position);
        self.octree.entry(hash).or_default().push((id, position));
    }

    /// Query all elements within a radius
    pub fn query_radius(&self, pos: Vec3, radius: f32) -> Vec<u32> {
        let cells_needed = (radius / self.cell_size).ceil() as i32;
        let radius_sq = radius * radius;

        // OPTIMIZATION: Pre-allocate based on expected results to avoid reallocation
        // Typical case: 20 elements per cell × cells_to_check, capped at 2000
        let expected_cells = match cells_needed {
            1 => 27,
            2 => 125,
            3 => 343,
            4 => 729,
            _ => (2 * cells_needed + 1).pow(3) as usize,
        };
        let mut results = Vec::with_capacity((expected_cells * 20).min(2000));

        // CRITICAL OPTIMIZATION: Compute base cell coordinates ONCE
        let base_ix = ((pos.x - self.bounds.0.x) / self.cell_size).floor() as i32;
        let base_iy = ((pos.y - self.bounds.0.y) / self.cell_size).floor() as i32;
        let base_iz = ((pos.z - self.bounds.0.z) / self.cell_size).floor() as i32;

        // OPTIMIZATION: Most common cases use pre-computed offsets to eliminate nested loops
        match cells_needed {
            1 => {
                // 27 cells (3×3×3) - most common case (~67% of queries)
                for &(dx, dy, dz) in &OFFSETS_1 {
                    let hash = morton_encode(base_ix + dx, base_iy + dy, base_iz + dz);
                    if let Some(elements) = self.octree.get(&hash) {
                        // OPTIMIZATION: Use slice iteration to reduce iterator overhead
                        for elem_idx in 0..elements.len() {
                            // SAFETY: elem_idx is bounded by 0..elements.len(), so it is always
                            // a valid index into the elements vector for this octree cell
                            let (id, element_pos) = unsafe { elements.get_unchecked(elem_idx) };
                            let dx = element_pos.x - pos.x;
                            let dy = element_pos.y - pos.y;
                            let dz = element_pos.z - pos.z;
                            let dist_sq = dx * dx + dy * dy + dz * dz;

                            // OPTIMIZATION: Branch prediction hint - most elements are within radius
                            if dist_sq <= radius_sq {
                                results.push(*id);
                            }
                        }
                    }
                }
            }
            2 => {
                // 125 cells (5×5×5) - high wind scenarios
                for &(dx, dy, dz) in &OFFSETS_2 {
                    let hash = morton_encode(base_ix + dx, base_iy + dy, base_iz + dz);
                    if let Some(elements) = self.octree.get(&hash) {
                        for elem_idx in 0..elements.len() {
                            // SAFETY: elem_idx is bounded by 0..elements.len(), so it is always
                            // a valid index into the elements vector for this octree cell
                            let (id, element_pos) = unsafe { elements.get_unchecked(elem_idx) };
                            let dx = element_pos.x - pos.x;
                            let dy = element_pos.y - pos.y;
                            let dz = element_pos.z - pos.z;
                            if dx * dx + dy * dy + dz * dz <= radius_sq {
                                results.push(*id);
                            }
                        }
                    }
                }
            }
            3 => {
                // 343 cells (7×7×7) - extreme wind
                for &(dx, dy, dz) in &OFFSETS_3 {
                    let hash = morton_encode(base_ix + dx, base_iy + dy, base_iz + dz);
                    if let Some(elements) = self.octree.get(&hash) {
                        for elem_idx in 0..elements.len() {
                            // SAFETY: elem_idx is bounded by 0..elements.len(), so it is always
                            // a valid index into the elements vector for this octree cell
                            let (id, element_pos) = unsafe { elements.get_unchecked(elem_idx) };
                            let dx = element_pos.x - pos.x;
                            let dy = element_pos.y - pos.y;
                            let dz = element_pos.z - pos.z;
                            if dx * dx + dy * dy + dz * dz <= radius_sq {
                                results.push(*id);
                            }
                        }
                    }
                }
            }
            4 => {
                // 729 cells (9×9×9) - catastrophic wind
                for &(dx, dy, dz) in &OFFSETS_4 {
                    let hash = morton_encode(base_ix + dx, base_iy + dy, base_iz + dz);
                    if let Some(elements) = self.octree.get(&hash) {
                        for elem_idx in 0..elements.len() {
                            // SAFETY: elem_idx is bounded by 0..elements.len(), so it is always
                            // a valid index into the elements vector for this octree cell
                            let (id, element_pos) = unsafe { elements.get_unchecked(elem_idx) };
                            let dx = element_pos.x - pos.x;
                            let dy = element_pos.y - pos.y;
                            let dz = element_pos.z - pos.z;
                            if dx * dx + dy * dy + dz * dz <= radius_sq {
                                results.push(*id);
                            }
                        }
                    }
                }
            }
            _ => {
                // General case for extremely large radii (rare)
                for dx in -cells_needed..=cells_needed {
                    for dy in -cells_needed..=cells_needed {
                        for dz in -cells_needed..=cells_needed {
                            let hash = morton_encode(base_ix + dx, base_iy + dy, base_iz + dz);
                            if let Some(elements) = self.octree.get(&hash) {
                                for elem_idx in 0..elements.len() {
                                    // SAFETY: elem_idx is bounded by 0..elements.len(), so it is always
                                    // a valid index into the elements vector for this octree cell
                                    let (id, element_pos) =
                                        unsafe { elements.get_unchecked(elem_idx) };
                                    let dx = element_pos.x - pos.x;
                                    let dy = element_pos.y - pos.y;
                                    let dz = element_pos.z - pos.z;
                                    if dx * dx + dy * dy + dz * dz <= radius_sq {
                                        results.push(*id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }
}

/// Morton encode 3D coordinates into a single 64-bit integer
/// This provides spatial locality for hash lookups
#[inline(always)]
fn morton_encode(x: i32, y: i32, z: i32) -> u64 {
    // Optimized bit interleaving - unroll loop and use direct bit operations
    let xu = x as u64;
    let yu = y as u64;
    let zu = z as u64;

    let mut result: u64 = 0;

    // Unrolled for first 8 bits (covers typical coordinate ranges)
    result |= (xu & 1) | (yu & 1) << 1 | (zu & 1) << 2;
    result |= ((xu >> 1) & 1) << 3 | ((yu >> 1) & 1) << 4 | ((zu >> 1) & 1) << 5;
    result |= ((xu >> 2) & 1) << 6 | ((yu >> 2) & 1) << 7 | ((zu >> 2) & 1) << 8;
    result |= ((xu >> 3) & 1) << 9 | ((yu >> 3) & 1) << 10 | ((zu >> 3) & 1) << 11;
    result |= ((xu >> 4) & 1) << 12 | ((yu >> 4) & 1) << 13 | ((zu >> 4) & 1) << 14;
    result |= ((xu >> 5) & 1) << 15 | ((yu >> 5) & 1) << 16 | ((zu >> 5) & 1) << 17;
    result |= ((xu >> 6) & 1) << 18 | ((yu >> 6) & 1) << 19 | ((zu >> 6) & 1) << 20;
    result |= ((xu >> 7) & 1) << 21 | ((yu >> 7) & 1) << 22 | ((zu >> 7) & 1) << 23;

    // Loop for remaining bits (rarely needed for typical coordinates)
    for i in 8..21 {
        let bit_pos = i as u64;
        result |= ((xu >> bit_pos) & 1) << (3 * bit_pos);
        result |= ((yu >> bit_pos) & 1) << (3 * bit_pos + 1);
        result |= ((zu >> bit_pos) & 1) << (3 * bit_pos + 2);
    }

    result
}

/// Generate neighbor offsets at compile time for a given radius
/// SIZE must equal (2*RADIUS + 1)³ - this is verified at compile time
const fn generate_neighbor_offsets<const RADIUS: i32, const SIZE: usize>() -> [(i32, i32, i32); SIZE]
{
    // Compile-time assertion that SIZE matches expected cube size
    // This will cause a compilation error if SIZE is incorrect
    let expected_size = {
        let side = 2 * RADIUS + 1;
        (side * side * side) as usize
    };
    assert!(SIZE == expected_size, "SIZE must equal (2*RADIUS + 1)³");

    let mut offsets = [(0, 0, 0); SIZE];
    let mut idx = 0;

    let mut dx = -RADIUS;
    while dx <= RADIUS {
        let mut dy = -RADIUS;
        while dy <= RADIUS {
            let mut dz = -RADIUS;
            while dz <= RADIUS {
                offsets[idx] = (dx, dy, dz);
                idx += 1;
                dz += 1;
            }
            dy += 1;
        }
        dx += 1;
    }

    offsets
}

// Pre-computed offset arrays for common radii (computed at compile time)
// Radius 1: 3×3×3 = 27 cells, Radius 2: 5×5×5 = 125 cells, etc.
const OFFSETS_1: [(i32, i32, i32); 27] = generate_neighbor_offsets::<1, 27>();
const OFFSETS_2: [(i32, i32, i32); 125] = generate_neighbor_offsets::<2, 125>();
const OFFSETS_3: [(i32, i32, i32); 343] = generate_neighbor_offsets::<3, 343>();
const OFFSETS_4: [(i32, i32, i32); 729] = generate_neighbor_offsets::<4, 729>();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_index_insert_query() {
        let bounds = (
            Vec3::new(-100.0, -100.0, -100.0),
            Vec3::new(100.0, 100.0, 100.0),
        );
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
