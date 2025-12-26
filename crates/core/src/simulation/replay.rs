//! Replay Data Capture and Playback System
//!
//! Enables match analysis and educational review of firefighting decisions.
//! Replays are saved with .bfsreplay extension and use zstd compression.

use crate::simulation::network::StateDelta;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Replay file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// File format version
    pub version: u32,
    /// Scenario name/description
    pub scenario_name: String,
    /// Simulation duration in seconds
    pub duration_seconds: f32,
    /// Total number of frames
    pub total_frames: u32,
    /// Recording timestamp
    pub recorded_at: DateTime<Utc>,
    /// Terrain dimensions
    pub terrain_width: f32,
    pub terrain_height: f32,
}

/// GPU state snapshot for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStateSnapshot {
    /// Frame number
    pub frame: u32,
    /// Simulation time
    pub sim_time: f32,
    /// Level set φ field state (compressed)
    pub phi_field: Vec<i32>, // Fixed-point (1000× scale)
    /// Fuel element states
    pub element_states: Vec<ElementState>,
    /// Wind field state (optional, may be deterministic)
    pub wind_field: Option<Vec<(f32, f32)>>, // (u, v) components
}

/// Fuel element state for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementState {
    /// Element ID
    pub id: usize,
    /// Temperature (Celsius, fixed-point 100× scale)
    pub temperature: i32,
    /// Moisture content (fixed-point 10000× scale)
    pub moisture: u16,
    /// Burning state
    pub is_burning: bool,
}

/// Replay file container
#[derive(Debug)]
pub struct ReplayFile {
    /// Metadata
    pub metadata: ReplayMetadata,
    /// Snapshots (sparse - not every frame)
    snapshots: Vec<GpuStateSnapshot>,
    /// Deltas between snapshots (compressed)
    deltas: Vec<StateDelta>,
}

impl ReplayFile {
    /// Create a new replay file
    pub fn new(scenario_name: String, terrain_width: f32, terrain_height: f32) -> Self {
        Self {
            metadata: ReplayMetadata {
                version: 1,
                scenario_name,
                duration_seconds: 0.0,
                total_frames: 0,
                recorded_at: Utc::now(),
                terrain_width,
                terrain_height,
            },
            snapshots: Vec::new(),
            deltas: Vec::new(),
        }
    }

    /// Add a state snapshot (keyframe)
    pub fn add_snapshot(&mut self, snapshot: GpuStateSnapshot) {
        self.metadata.total_frames = snapshot.frame;
        self.metadata.duration_seconds = snapshot.sim_time;
        self.snapshots.push(snapshot);
    }

    /// Add a state delta (incremental change)
    pub fn add_delta(&mut self, delta: StateDelta) {
        self.deltas.push(delta);
    }

    /// Save replay to file with zstd compression
    ///
    /// # Errors
    ///
    /// Returns an error if file I/O fails or serialization fails.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Serialize to JSON
        let json = serde_json::to_string(&(&self.metadata, &self.snapshots, &self.deltas))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Compress with zstd (level 9 for high ratio)
        let compressed = zstd::encode_all(json.as_bytes(), 9)?;

        writer.write_all(&compressed)?;
        writer.flush()?;

        Ok(())
    }

    /// Load replay from file
    ///
    /// # Errors
    ///
    /// Returns an error if file I/O fails, decompression fails, or deserialization fails.
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read compressed data
        let mut compressed = Vec::new();
        reader.read_to_end(&mut compressed)?;

        // Decompress
        let decompressed = zstd::decode_all(&compressed[..])?;

        // Deserialize from JSON
        let (metadata, snapshots, deltas) = serde_json::from_slice(&decompressed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            metadata,
            snapshots,
            deltas,
        })
    }

    /// Get snapshot at specific frame (interpolates if needed)
    pub fn get_snapshot_at_frame(&self, frame: u32) -> Option<&GpuStateSnapshot> {
        // Find closest snapshot <= frame
        self.snapshots.iter().rev().find(|s| s.frame <= frame)
    }

    /// Get all deltas between two frames
    pub fn get_deltas_between(&self, start_frame: u32, end_frame: u32) -> Vec<&StateDelta> {
        self.deltas
            .iter()
            .filter(|d| d.frame >= start_frame && d.frame <= end_frame)
            .collect()
    }

    /// Get total number of frames
    pub fn total_frames(&self) -> u32 {
        self.metadata.total_frames
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> f32 {
        self.metadata.duration_seconds
    }
}

/// Replay player state machine
#[derive(Debug)]
pub struct ReplayPlayer {
    /// Loaded replay file
    replay: ReplayFile,
    /// Current playback frame
    current_frame: u32,
    /// Playback speed multiplier
    speed: f32,
    /// Paused state
    paused: bool,
}

impl ReplayPlayer {
    /// Create a new replay player
    pub fn new(replay: ReplayFile) -> Self {
        Self {
            replay,
            current_frame: 0,
            speed: 1.0,
            paused: false,
        }
    }

    /// Step to a specific frame
    pub fn step_to_frame(&mut self, frame: u32) -> Option<&GpuStateSnapshot> {
        self.current_frame = frame.min(self.replay.total_frames());
        self.replay.get_snapshot_at_frame(self.current_frame)
    }

    /// Advance by one frame
    pub fn step_forward(&mut self) -> Option<&GpuStateSnapshot> {
        if self.current_frame < self.replay.total_frames() {
            self.current_frame += 1;
            self.replay.get_snapshot_at_frame(self.current_frame)
        } else {
            None
        }
    }

    /// Go back one frame
    pub fn step_backward(&mut self) -> Option<&GpuStateSnapshot> {
        if self.current_frame > 0 {
            self.current_frame -= 1;
            self.replay.get_snapshot_at_frame(self.current_frame)
        } else {
            None
        }
    }

    /// Get current frame number
    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    /// Get total frames
    pub fn total_frames(&self) -> u32 {
        self.replay.total_frames()
    }

    /// Set playback speed
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.1, 10.0); // Clamp to reasonable range
    }

    /// Get playback speed
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume playback
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.paused = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_file_creation() {
        let replay = ReplayFile::new("Test Scenario".to_string(), 1000.0, 1000.0);
        assert_eq!(replay.metadata.scenario_name, "Test Scenario");
        assert_eq!(replay.metadata.version, 1);
        assert_eq!(replay.total_frames(), 0);
    }

    #[test]
    fn test_add_snapshot() {
        let mut replay = ReplayFile::new("Test".to_string(), 1000.0, 1000.0);

        let snapshot = GpuStateSnapshot {
            frame: 100,
            sim_time: 10.0,
            phi_field: vec![1000, 2000, 3000],
            element_states: vec![],
            wind_field: None,
        };

        replay.add_snapshot(snapshot);
        assert_eq!(replay.total_frames(), 100);
        assert_eq!(replay.duration_seconds(), 10.0);
    }

    #[test]
    fn test_save_load_round_trip() -> io::Result<()> {
        let mut replay = ReplayFile::new("Round Trip Test".to_string(), 1000.0, 1000.0);

        let snapshot = GpuStateSnapshot {
            frame: 50,
            sim_time: 5.0,
            phi_field: vec![1000, 2000, 3000],
            element_states: vec![ElementState {
                id: 1,
                temperature: 60000, // 600°C × 100
                moisture: 500,      // 0.05 × 10000
                is_burning: true,
            }],
            wind_field: Some(vec![(1.0, 2.0), (3.0, 4.0)]),
        };

        replay.add_snapshot(snapshot);

        // Save to temp file
        let temp_path = "/tmp/test_replay.bfsreplay";
        replay.save(temp_path)?;

        // Load and verify
        let loaded = ReplayFile::load(temp_path)?;
        assert_eq!(loaded.metadata.scenario_name, "Round Trip Test");
        assert_eq!(loaded.total_frames(), 50);
        assert_eq!(loaded.snapshots.len(), 1);
        assert_eq!(loaded.snapshots[0].phi_field, vec![1000, 2000, 3000]);

        // Clean up
        std::fs::remove_file(temp_path)?;

        Ok(())
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Test code using small integers for readability; precision loss acceptable"
    )]
    fn test_replay_player() {
        let mut replay = ReplayFile::new("Player Test".to_string(), 1000.0, 1000.0);

        // Add snapshots at frames 0, 10, 20
        for i in 0..3 {
            let snapshot = GpuStateSnapshot {
                frame: i * 10,
                sim_time: i as f32 * 1.0,
                phi_field: vec![i as i32 * 1000],
                element_states: vec![],
                wind_field: None,
            };
            replay.add_snapshot(snapshot);
        }

        let mut player = ReplayPlayer::new(replay);

        assert_eq!(player.current_frame(), 0);
        assert_eq!(player.total_frames(), 20);

        // Step forward
        player.step_forward();
        assert_eq!(player.current_frame(), 1);

        // Jump to frame
        player.step_to_frame(15);
        assert_eq!(player.current_frame(), 15);

        // Step backward
        player.step_backward();
        assert_eq!(player.current_frame(), 14);

        // Speed control
        player.set_speed(2.0);
        assert_eq!(player.speed(), 2.0);

        // Pause/resume
        assert!(!player.is_paused());
        player.pause();
        assert!(player.is_paused());
        player.resume();
        assert!(!player.is_paused());

        // Reset
        player.reset();
        assert_eq!(player.current_frame(), 0);
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Test code using small integers for readability; precision loss acceptable"
    )]
    fn test_get_snapshot_at_frame() {
        let mut replay = ReplayFile::new("Snapshot Test".to_string(), 1000.0, 1000.0);

        // Add snapshots at frames 0, 10, 20
        for i in 0..3 {
            let snapshot = GpuStateSnapshot {
                frame: i * 10,
                sim_time: i as f32 * 1.0,
                phi_field: vec![i as i32 * 1000],
                element_states: vec![],
                wind_field: None,
            };
            replay.add_snapshot(snapshot);
        }

        // Query frame 5 should return snapshot at frame 0
        let snap = replay.get_snapshot_at_frame(5).unwrap();
        assert_eq!(snap.frame, 0);

        // Query frame 15 should return snapshot at frame 10
        let snap = replay.get_snapshot_at_frame(15).unwrap();
        assert_eq!(snap.frame, 10);

        // Query frame 20 should return snapshot at frame 20
        let snap = replay.get_snapshot_at_frame(20).unwrap();
        assert_eq!(snap.frame, 20);
    }
}
