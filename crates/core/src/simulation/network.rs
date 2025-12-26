//! Multiplayer network state synchronization with delta compression
//!
//! This module provides delta compression for efficient multiplayer state sync.
//! Target: <100KB per frame for 10km² fire simulation.

use bitvec::prelude::*;
use std::io::{self, Write};

/// Network state delta for multiplayer synchronization
#[derive(Debug, Clone)]
pub struct StateDelta {
    /// Dirty tile flags (64×64 tiles for a 2048×2048 grid)
    pub dirty_tiles: BitVec,
    /// Changed phi field values (run-length encoded)
    pub phi_changes: Vec<PhiChange>,
    /// Changed fuel element states
    pub element_changes: Vec<ElementChange>,
    /// Frame number this delta represents
    pub frame: u32,
}

/// A change in the phi field (level set signed distance)
#[derive(Debug, Clone)]
pub struct PhiChange {
    /// Tile index (0-4095 for 64×64 grid of tiles)
    pub tile_idx: u16,
    /// Phi values for this tile (compressed)
    pub values: Vec<i32>, // Fixed-point representation (1000× scale)
}

/// A change in a fuel element state
#[derive(Debug, Clone)]
pub struct ElementChange {
    /// Element ID
    pub element_id: usize,
    /// New temperature (fixed-point, 100× scale)
    pub temperature: i32,
    /// New moisture content (fixed-point, 10000× scale)
    pub moisture: u16,
    /// Burning state
    pub is_burning: bool,
}

impl StateDelta {
    /// Create a new empty delta
    pub fn new(frame: u32) -> Self {
        // For a 2048×2048 grid with 32×32 pixel tiles, we have 64×64 tiles
        let tile_count = 64 * 64;
        Self {
            dirty_tiles: bitvec![0; tile_count],
            phi_changes: Vec::new(),
            element_changes: Vec::new(),
            frame,
        }
    }

    /// Mark a tile as dirty (modified this frame)
    pub fn mark_tile_dirty(&mut self, tile_x: u32, tile_y: u32) {
        let tile_idx = tile_y * 64 + tile_x;
        if (tile_idx as usize) < self.dirty_tiles.len() {
            self.dirty_tiles.set(tile_idx as usize, true);
        }
    }

    /// Check if a tile is dirty
    pub fn is_tile_dirty(&self, tile_x: u32, tile_y: u32) -> bool {
        let tile_idx = tile_y * 64 + tile_x;
        if (tile_idx as usize) < self.dirty_tiles.len() {
            self.dirty_tiles[tile_idx as usize]
        } else {
            false
        }
    }

    /// Add a phi change for a tile
    pub fn add_phi_change(&mut self, tile_idx: u16, values: Vec<i32>) {
        self.phi_changes.push(PhiChange { tile_idx, values });
    }

    /// Add an element state change
    pub fn add_element_change(&mut self, change: ElementChange) {
        self.element_changes.push(change);
    }

    /// Serialize delta to bytes with zstd compression
    ///
    /// # Errors
    /// Returns I/O error if compression fails
    pub fn serialize_compressed(&self) -> io::Result<Vec<u8>> {
        // Serialize to uncompressed format first
        let uncompressed = self.serialize_uncompressed()?;

        // Compress with zstd (level 3 for speed/compression balance)
        let compressed = zstd::encode_all(&uncompressed[..], 3)?;

        Ok(compressed)
    }

    /// Serialize delta to uncompressed bytes
    fn serialize_uncompressed(&self) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Write header: frame number
        buffer.write_all(&self.frame.to_le_bytes())?;

        // Write dirty tile bitmap (run-length encoded)
        let rle_tiles = self.encode_dirty_tiles_rle();
        buffer.write_all(&(rle_tiles.len() as u32).to_le_bytes())?;
        buffer.write_all(&rle_tiles)?;

        // Write phi changes
        buffer.write_all(&(self.phi_changes.len() as u32).to_le_bytes())?;
        for change in &self.phi_changes {
            buffer.write_all(&change.tile_idx.to_le_bytes())?;
            buffer.write_all(&(change.values.len() as u32).to_le_bytes())?;
            for value in &change.values {
                buffer.write_all(&value.to_le_bytes())?;
            }
        }

        // Write element changes
        buffer.write_all(&(self.element_changes.len() as u32).to_le_bytes())?;
        for change in &self.element_changes {
            buffer.write_all(&(change.element_id as u32).to_le_bytes())?;
            buffer.write_all(&change.temperature.to_le_bytes())?;
            buffer.write_all(&change.moisture.to_le_bytes())?;
            buffer.write_all(&[u8::from(change.is_burning)])?;
        }

        Ok(buffer)
    }

    /// Encode dirty tiles bitmap as run-length encoded data
    fn encode_dirty_tiles_rle(&self) -> Vec<u8> {
        let mut rle = Vec::new();
        let mut run_value = false;
        let mut run_length: u16 = 0;

        for bit in &self.dirty_tiles {
            let bit_val = *bit;
            if bit_val == run_value {
                run_length += 1;
                if run_length == u16::MAX {
                    // Flush run
                    rle.push(u8::from(run_value));
                    rle.extend_from_slice(&run_length.to_le_bytes());
                    run_length = 0;
                }
            } else {
                // Flush previous run
                if run_length > 0 {
                    rle.push(u8::from(run_value));
                    rle.extend_from_slice(&run_length.to_le_bytes());
                }
                run_value = bit_val;
                run_length = 1;
            }
        }

        // Flush final run
        if run_length > 0 {
            rle.push(u8::from(run_value));
            rle.extend_from_slice(&run_length.to_le_bytes());
        }

        rle
    }

    /// Get estimated compressed size in bytes
    pub fn estimated_size(&self) -> usize {
        // Rough estimate: header + dirty tiles + phi changes + element changes
        let base = 4; // frame number
        let tiles = self.dirty_tiles.count_ones() * 4; // Approximate per dirty tile
        let phi = self
            .phi_changes
            .iter()
            .map(|c| 2 + 4 + c.values.len() * 4)
            .sum::<usize>();
        let elements = self.element_changes.len() * (4 + 4 + 2 + 1);

        // Assume ~3× compression with zstd
        (base + tiles + phi + elements) / 3
    }
}

/// State delta builder for tracking changes during a frame
#[allow(dead_code)] // grid_width/grid_height reserved for future bounds checking
pub struct StateDeltaBuilder {
    delta: StateDelta,
    tile_size: u32,
    grid_width: u32,
    grid_height: u32,
}

impl StateDeltaBuilder {
    /// Create a new delta builder
    pub fn new(frame: u32, grid_width: u32, grid_height: u32) -> Self {
        Self {
            delta: StateDelta::new(frame),
            tile_size: 32, // 32×32 pixel tiles
            grid_width,
            grid_height,
        }
    }

    /// Track a change in phi field at grid position
    pub fn track_phi_change(&mut self, grid_x: u32, grid_y: u32) {
        let tile_x = grid_x / self.tile_size;
        let tile_y = grid_y / self.tile_size;
        self.delta.mark_tile_dirty(tile_x, tile_y);
    }

    /// Track an element state change
    pub fn track_element_change(&mut self, change: ElementChange) {
        self.delta.add_element_change(change);
    }

    /// Build the final delta
    pub fn build(self) -> StateDelta {
        self.delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_creation() {
        let delta = StateDelta::new(42);
        assert_eq!(delta.frame, 42);
        assert_eq!(delta.dirty_tiles.len(), 64 * 64);
        assert_eq!(delta.phi_changes.len(), 0);
        assert_eq!(delta.element_changes.len(), 0);
    }

    #[test]
    fn test_mark_tile_dirty() {
        let mut delta = StateDelta::new(0);
        delta.mark_tile_dirty(5, 10);
        assert!(delta.is_tile_dirty(5, 10));
        assert!(!delta.is_tile_dirty(5, 11));
    }

    #[test]
    fn test_add_changes() {
        let mut delta = StateDelta::new(0);

        delta.add_phi_change(100, vec![1000, 2000, 3000]);
        delta.add_element_change(ElementChange {
            element_id: 42,
            temperature: 60000, // 600.00°C
            moisture: 1500,     // 0.1500
            is_burning: true,
        });

        assert_eq!(delta.phi_changes.len(), 1);
        assert_eq!(delta.element_changes.len(), 1);
        assert_eq!(delta.element_changes[0].element_id, 42);
    }

    #[test]
    fn test_serialization() {
        let mut delta = StateDelta::new(123);
        delta.mark_tile_dirty(0, 0);
        delta.add_phi_change(0, vec![1000, 2000]);
        delta.add_element_change(ElementChange {
            element_id: 5,
            temperature: 50000,
            moisture: 2000,
            is_burning: true,
        });

        let serialized = delta.serialize_uncompressed().unwrap();
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_compression() {
        let mut delta = StateDelta::new(123);

        // Add some realistic data
        for i in 0..10 {
            delta.mark_tile_dirty(i, i);
            delta.add_phi_change(i as u16, vec![1000 * i as i32; 32 * 32]);
        }

        let compressed = delta.serialize_compressed().unwrap();
        let uncompressed = delta.serialize_uncompressed().unwrap();

        // Compression should reduce size
        assert!(compressed.len() < uncompressed.len());
    }

    #[test]
    fn test_delta_builder() {
        let mut builder = StateDeltaBuilder::new(1, 2048, 2048);

        builder.track_phi_change(100, 200);
        builder.track_element_change(ElementChange {
            element_id: 7,
            temperature: 40000,
            moisture: 3000,
            is_burning: false,
        });

        let delta = builder.build();
        assert!(delta.is_tile_dirty(100 / 32, 200 / 32));
        assert_eq!(delta.element_changes.len(), 1);
    }

    #[test]
    fn test_estimated_size() {
        let mut delta = StateDelta::new(0);
        delta.add_phi_change(0, vec![1000; 100]);
        delta.add_element_change(ElementChange {
            element_id: 1,
            temperature: 50000,
            moisture: 2000,
            is_burning: true,
        });

        let estimated = delta.estimated_size();
        assert!(estimated > 0);
        assert!(estimated < 10000); // Should be reasonable
    }
}
