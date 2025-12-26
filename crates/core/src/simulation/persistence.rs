//! Persistent world damage tracking across gaming sessions
//!
//! This module provides optional persistent world state where fire damage
//! accumulates across multiple missions and vegetation recovers over time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Persistent world state tracking fuel consumption and recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentWorldState {
    /// Grid of remaining fuel at each position (0.0-1.0 where 1.0 = full fuel)
    pub fuel_remaining: Vec<f32>,
    /// Grid width
    pub width: usize,
    /// Grid height
    pub height: usize,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
    /// Total area burned in hectares
    pub total_burned_hectares: f32,
}

impl PersistentWorldState {
    /// Create a new persistent world state
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            fuel_remaining: vec![1.0; width * height],
            width,
            height,
            last_update: Utc::now(),
            total_burned_hectares: 0.0,
        }
    }

    /// Load persistent state from file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PersistenceError> {
        let contents =
            fs::read_to_string(path).map_err(|e| PersistenceError::LoadFailed(e.to_string()))?;

        let state: Self = serde_json::from_str(&contents)
            .map_err(|e| PersistenceError::ParseFailed(e.to_string()))?;

        Ok(state)
    }

    /// Save persistent state to file
    ///
    /// # Errors
    /// Returns error if file cannot be written or state cannot be serialized
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), PersistenceError> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| PersistenceError::SerializeFailed(e.to_string()))?;

        fs::write(path, contents).map_err(|e| PersistenceError::SaveFailed(e.to_string()))?;

        Ok(())
    }

    /// Apply fire damage to a grid cell
    pub fn apply_damage(&mut self, x: usize, y: usize, damage: f32) {
        let idx = y * self.width + x;
        if idx < self.fuel_remaining.len() {
            self.fuel_remaining[idx] = (self.fuel_remaining[idx] - damage).max(0.0);
        }
    }

    /// Get fuel remaining at position
    pub fn get_fuel_remaining(&self, x: usize, y: usize) -> f32 {
        let idx = y * self.width + x;
        self.fuel_remaining.get(idx).copied().unwrap_or(1.0)
    }

    /// Update recovery based on elapsed time
    #[expect(
        clippy::cast_precision_loss,
        reason = "Days to years conversion - precision loss acceptable for recovery rate"
    )]
    pub fn update_recovery(&mut self, current_time: DateTime<Utc>) {
        let years_elapsed = (current_time - self.last_update).num_days() as f32 / 365.25;

        // Recovery rate: 10% per year
        let recovery_rate = 0.10;
        let recovery_amount = recovery_rate * years_elapsed;

        for fuel in &mut self.fuel_remaining {
            *fuel = (*fuel + recovery_amount).min(1.0);
        }

        self.last_update = current_time;
    }

    /// Calculate total burned area in hectares
    pub fn calculate_burned_area(&self, cell_size_meters: f32) -> f32 {
        let cell_area_sq_m = cell_size_meters * cell_size_meters;
        let cells_burned = self
            .fuel_remaining
            .iter()
            .filter(|&&f| f < 0.9) // Cells with >10% damage
            .count();

        // Convert to hectares (10,000 m² = 1 hectare)
        #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for area calculation
        {
            (cells_burned as f32 * cell_area_sq_m) / 10000.0
        }
    }

    /// Reset all fuel to full
    pub fn reset(&mut self) {
        for fuel in &mut self.fuel_remaining {
            *fuel = 1.0;
        }
        self.total_burned_hectares = 0.0;
        self.last_update = Utc::now();
    }
}

/// Errors that can occur with persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    /// Failed to load file
    LoadFailed(String),
    /// Failed to parse file contents
    ParseFailed(String),
    /// Failed to serialize state
    SerializeFailed(String),
    /// Failed to save file
    SaveFailed(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::LoadFailed(msg) => write!(f, "Failed to load: {msg}"),
            PersistenceError::ParseFailed(msg) => write!(f, "Failed to parse: {msg}"),
            PersistenceError::SerializeFailed(msg) => write!(f, "Failed to serialize: {msg}"),
            PersistenceError::SaveFailed(msg) => write!(f, "Failed to save: {msg}"),
        }
    }
}

impl std::error::Error for PersistenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_world_state() {
        let state = PersistentWorldState::new(100, 100);
        assert_eq!(state.width, 100);
        assert_eq!(state.height, 100);
        assert_eq!(state.fuel_remaining.len(), 10000);
        assert_eq!(state.fuel_remaining[0], 1.0);
    }

    #[test]
    fn test_apply_damage() {
        let mut state = PersistentWorldState::new(10, 10);

        state.apply_damage(5, 5, 0.3);
        assert!((state.get_fuel_remaining(5, 5) - 0.7).abs() < 1e-6);

        // Apply more damage
        state.apply_damage(5, 5, 0.5);
        assert!((state.get_fuel_remaining(5, 5) - 0.2).abs() < 1e-6);

        // Can't go below 0
        state.apply_damage(5, 5, 1.0);
        assert!((state.get_fuel_remaining(5, 5) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_recovery() {
        let mut state = PersistentWorldState::new(10, 10);

        // Damage a cell
        state.apply_damage(0, 0, 0.5);
        assert_eq!(state.get_fuel_remaining(0, 0), 0.5);

        // Simulate 1 year passing
        let one_year_later = state.last_update + chrono::Duration::days(365);
        state.update_recovery(one_year_later);

        // Should have recovered by 10%
        assert!((state.get_fuel_remaining(0, 0) - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_burned_area_calculation() {
        let mut state = PersistentWorldState::new(100, 100);

        // Damage 100 cells (each 5m × 5m = 25 m²)
        for i in 0..100 {
            state.apply_damage(i % 100, i / 100, 0.5);
        }

        let burned_area = state.calculate_burned_area(5.0);

        // 100 cells × 25 m² = 2500 m² = 0.25 hectares
        assert!((burned_area - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut state = PersistentWorldState::new(10, 10);

        state.apply_damage(0, 0, 1.0);
        state.total_burned_hectares = 100.0;

        state.reset();

        assert_eq!(state.get_fuel_remaining(0, 0), 1.0);
        assert_eq!(state.total_burned_hectares, 0.0);
    }

    #[test]
    fn test_save_and_load() {
        let mut state = PersistentWorldState::new(10, 10);
        state.apply_damage(5, 5, 0.3);
        state.total_burned_hectares = 10.0;

        let temp_path = "/tmp/test_persistence.json";

        // Save
        state.save(temp_path).unwrap();

        // Load
        let loaded = PersistentWorldState::load(temp_path).unwrap();

        assert_eq!(loaded.width, state.width);
        assert_eq!(loaded.height, state.height);
        assert_eq!(loaded.get_fuel_remaining(5, 5), 0.7);
        assert_eq!(loaded.total_burned_hectares, 10.0);

        // Cleanup
        let _ = fs::remove_file(temp_path);
    }
}
