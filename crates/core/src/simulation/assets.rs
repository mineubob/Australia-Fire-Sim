//! Asset threat assessment for tactical firefighting decisions
//!
//! This module provides real-time threat assessment for registered assets
//! (buildings, infrastructure, etc.) based on fire arrival time predictions.

use crate::core_types::element::Vec3;
use crate::gpu::arrival_time::ArrivalPrediction;

/// An asset to protect from fire
#[derive(Debug, Clone)]
pub struct Asset {
    /// Asset position in world coordinates
    pub position: Vec3,
    /// Asset monetary or strategic value
    pub value: f32,
    /// Asset type for categorization
    pub asset_type: AssetType,
    /// Whether this is a critical asset (hospital, emergency services, etc.)
    pub critical: bool,
}

/// Type of asset for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    /// Residential building
    Residential,
    /// Commercial building
    Commercial,
    /// Industrial facility
    Industrial,
    /// Infrastructure (power, water, etc.)
    Infrastructure,
    /// Emergency services (fire station, hospital, etc.)
    EmergencyServices,
    /// Natural/cultural heritage site
    Heritage,
}

/// Threat level classification based on arrival time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    /// Fire will arrive in <5 minutes
    Immediate,
    /// Fire will arrive in 5-15 minutes
    High,
    /// Fire will arrive in 15-30 minutes
    Moderate,
    /// Fire will arrive in >30 minutes
    Low,
    /// Fire not expected to reach asset
    None,
}

impl ThreatLevel {
    /// Classify threat level from arrival time in seconds
    pub fn from_arrival_time(arrival_time_sec: Option<f32>) -> Self {
        match arrival_time_sec {
            None => ThreatLevel::None,
            Some(t) if t < 300.0 => ThreatLevel::Immediate, // <5 min
            Some(t) if t < 900.0 => ThreatLevel::High,      // <15 min
            Some(t) if t < 1800.0 => ThreatLevel::Moderate, // <30 min
            Some(_) => ThreatLevel::Low,                    // >30 min
        }
    }
}

/// Threat assessment for a single asset
#[derive(Debug, Clone)]
pub struct AssetThreat {
    /// Asset ID
    pub asset_id: usize,
    /// Asset information
    pub asset: Asset,
    /// Fire arrival prediction
    pub prediction: ArrivalPrediction,
    /// Threat level classification
    pub threat_level: ThreatLevel,
    /// Priority score (higher = more urgent)
    pub priority: f32,
}

impl AssetThreat {
    /// Create a new asset threat assessment
    pub fn new(asset_id: usize, asset: Asset, prediction: ArrivalPrediction) -> Self {
        let threat_level = ThreatLevel::from_arrival_time(prediction.arrival_time);
        let priority = Self::calculate_priority(&asset, &threat_level, prediction.arrival_time);

        Self {
            asset_id,
            asset,
            prediction,
            threat_level,
            priority,
        }
    }

    /// Calculate priority score
    fn calculate_priority(
        asset: &Asset,
        threat_level: &ThreatLevel,
        arrival_time: Option<f32>,
    ) -> f32 {
        let mut priority = 0.0;

        // Critical assets get massive priority boost
        if asset.critical {
            priority += 10000.0;
        }

        // Threat level scoring
        priority += match threat_level {
            ThreatLevel::Immediate => 1000.0,
            ThreatLevel::High => 500.0,
            ThreatLevel::Moderate => 100.0,
            ThreatLevel::Low => 10.0,
            ThreatLevel::None => 0.0,
        };

        // Value contribution (normalized to 0-100)
        priority += (asset.value / 1000000.0).min(100.0);

        // Time urgency (inverse of arrival time)
        if let Some(time) = arrival_time {
            priority += 1000.0 / (time + 1.0);
        }

        priority
    }
}

/// Asset registry for threat tracking
pub struct AssetRegistry {
    assets: Vec<Asset>,
    next_id: usize,
}

impl AssetRegistry {
    /// Create a new empty asset registry
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a new asset
    pub fn register(&mut self, asset: Asset) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.assets.push(asset);
        id
    }

    /// Remove an asset by ID
    pub fn remove(&mut self, id: usize) -> Option<Asset> {
        if let Some(idx) = self.assets.iter().position(|_| true) {
            // Note: This is simplified; proper implementation would track ID->index mapping
            if idx == id && idx < self.assets.len() {
                return Some(self.assets.remove(idx));
            }
        }
        None
    }

    /// Get all registered assets
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    /// Get asset count
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level_classification() {
        assert_eq!(ThreatLevel::from_arrival_time(None), ThreatLevel::None);
        assert_eq!(
            ThreatLevel::from_arrival_time(Some(100.0)),
            ThreatLevel::Immediate
        );
        assert_eq!(
            ThreatLevel::from_arrival_time(Some(600.0)),
            ThreatLevel::High
        );
        assert_eq!(
            ThreatLevel::from_arrival_time(Some(1200.0)),
            ThreatLevel::Moderate
        );
        assert_eq!(
            ThreatLevel::from_arrival_time(Some(2000.0)),
            ThreatLevel::Low
        );
    }

    #[test]
    fn test_asset_creation() {
        let asset = Asset {
            position: Vec3::new(100.0, 200.0, 0.0),
            value: 500000.0,
            asset_type: AssetType::Residential,
            critical: false,
        };

        assert_eq!(asset.position, Vec3::new(100.0, 200.0, 0.0));
        assert_eq!(asset.value, 500000.0);
        assert!(!asset.critical);
    }

    #[test]
    fn test_asset_registry() {
        let mut registry = AssetRegistry::new();

        let asset1 = Asset {
            position: Vec3::new(100.0, 100.0, 0.0),
            value: 100000.0,
            asset_type: AssetType::Residential,
            critical: false,
        };

        let id1 = registry.register(asset1.clone());
        assert_eq!(id1, 0);
        assert_eq!(registry.len(), 1);

        let asset2 = Asset {
            position: Vec3::new(200.0, 200.0, 0.0),
            value: 200000.0,
            asset_type: AssetType::Commercial,
            critical: true,
        };

        let id2 = registry.register(asset2);
        assert_eq!(id2, 1);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_priority_calculation() {
        let critical_asset = Asset {
            position: Vec3::new(0.0, 0.0, 0.0),
            value: 1000000.0,
            asset_type: AssetType::EmergencyServices,
            critical: true,
        };

        let normal_asset = Asset {
            position: Vec3::new(0.0, 0.0, 0.0),
            value: 100000.0,
            asset_type: AssetType::Residential,
            critical: false,
        };

        let pred = ArrivalPrediction {
            arrival_time: Some(200.0),
            distance_to_front: 100.0,
            avg_spread_rate: 0.5,
        };

        let critical_threat = AssetThreat::new(0, critical_asset, pred.clone());
        let normal_threat = AssetThreat::new(1, normal_asset, pred);

        // Critical asset should have much higher priority
        assert!(critical_threat.priority > normal_threat.priority);
        assert!(critical_threat.priority > 10000.0); // Has critical boost
    }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Immediate > ThreatLevel::High);
        assert!(ThreatLevel::High > ThreatLevel::Moderate);
        assert!(ThreatLevel::Moderate > ThreatLevel::Low);
        assert!(ThreatLevel::Low > ThreatLevel::None);
    }
}
