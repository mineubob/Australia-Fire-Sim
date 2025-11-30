//! Multi-layer Canopy Fire Transition Model
//!
//! Models vertical fire spread through forest canopy layers based on fuel
//! structure and fire intensity. Critical for simulating crown fire transitions
//! in Australian eucalypt forests.
//!
//! # Scientific References
//!
//! - Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire"
//!   Canadian Journal of Forest Research, 7(1), 23-34
//! - Cruz, M.G., et al. (2006). "Development and testing of models for predicting
//!   crown fire rate of spread in conifer forest stands"
//!   Canadian Journal of Forest Research, 36(6), 1614-1630
//! - Cheney, N.P., et al. (2012). "Predicting fire behaviour in dry eucalypt forest
//!   in southern Australia"
//!   Forest Ecology and Management, 280, 120-131

/// Vertical canopy layers for fire transition modeling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanopyLayer {
    /// Ground level: grass, litter (0-2m)
    Understory,
    /// Mid-level: shrubs, bark strips, ladder fuels (2-8m)
    Midstory,
    /// Upper: crown, foliage (8m+)
    Overstory,
}

impl CanopyLayer {
    /// Get height range for this layer in meters
    pub fn height_range(&self) -> (f32, f32) {
        match self {
            CanopyLayer::Understory => (0.0, 2.0),
            CanopyLayer::Midstory => (2.0, 8.0),
            CanopyLayer::Overstory => (8.0, 50.0),
        }
    }

    /// Check if a height falls within this layer
    pub fn contains_height(&self, height: f32) -> bool {
        let (min, max) = self.height_range();
        height >= min && height < max
    }
}

/// Canopy structure properties for a fuel type
#[derive(Debug, Clone)]
pub struct CanopyStructure {
    /// Fuel load by layer (kg/m²)
    pub understory_load: f32,
    pub midstory_load: f32,
    pub overstory_load: f32,

    /// Bulk density by layer (kg/m³)
    pub understory_density: f32,
    pub midstory_density: f32,
    pub overstory_density: f32,

    /// Moisture content by layer (fraction 0-1)
    pub understory_moisture: f32,
    pub midstory_moisture: f32,
    pub overstory_moisture: f32,

    /// Vertical fuel continuity (0-1, how connected layers are)
    ladder_fuel_factor: f32,
}

impl CanopyStructure {
    /// Get the ladder fuel factor
    pub fn ladder_fuel_factor(&self) -> f32 {
        self.ladder_fuel_factor
    }

    /// Create canopy structure for eucalyptus stringybark forest
    ///
    /// Stringybark has strong vertical continuity due to:
    /// - Fibrous bark hanging from trunk (ladder fuel)
    /// - Low crown base height
    /// - Dense mid-story shrubs
    pub fn eucalyptus_stringybark() -> Self {
        CanopyStructure {
            understory_load: 1.5, // kg/m² (grass, litter)
            midstory_load: 3.0,   // kg/m² (shrubs, bark strips)
            overstory_load: 4.5,  // kg/m² (canopy)

            understory_density: 0.3, // kg/m³
            midstory_density: 0.5,   // kg/m³ (includes bark ladder fuels)
            overstory_density: 0.2,  // kg/m³

            understory_moisture: 0.10, // Dry surface fuels
            midstory_moisture: 0.15,   // Dead bark strips
            overstory_moisture: 0.90,  // Live foliage

            ladder_fuel_factor: 0.9, // Very high continuity (stringybark!)
        }
    }

    /// Create canopy structure for eucalyptus smooth bark forest
    ///
    /// Smooth bark has less vertical continuity:
    /// - Minimal ladder fuels
    /// - Higher crown base
    /// - Gaps between layers
    pub fn eucalyptus_smooth_bark() -> Self {
        CanopyStructure {
            understory_load: 1.2,
            midstory_load: 1.0, // Less midstory
            overstory_load: 4.0,

            understory_density: 0.25,
            midstory_density: 0.15, // Sparse midstory
            overstory_density: 0.15,

            understory_moisture: 0.10,
            midstory_moisture: 0.20,
            overstory_moisture: 0.95,

            ladder_fuel_factor: 0.3, // Low continuity
        }
    }

    /// Create canopy structure for grassland (single layer)
    pub fn grassland() -> Self {
        CanopyStructure {
            understory_load: 0.8,
            midstory_load: 0.0,
            overstory_load: 0.0,

            understory_density: 0.2,
            midstory_density: 0.0,
            overstory_density: 0.0,

            understory_moisture: 0.05,
            midstory_moisture: 0.0,
            overstory_moisture: 0.0,

            ladder_fuel_factor: 0.0, // No vertical structure
        }
    }

    /// Get fuel load for a specific layer
    pub fn load_at_layer(&self, layer: CanopyLayer) -> f32 {
        match layer {
            CanopyLayer::Understory => self.understory_load,
            CanopyLayer::Midstory => self.midstory_load,
            CanopyLayer::Overstory => self.overstory_load,
        }
    }

    /// Get bulk density for a specific layer
    pub fn density_at_layer(&self, layer: CanopyLayer) -> f32 {
        match layer {
            CanopyLayer::Understory => self.understory_density,
            CanopyLayer::Midstory => self.midstory_density,
            CanopyLayer::Overstory => self.overstory_density,
        }
    }

    /// Get moisture for a specific layer
    pub fn moisture_at_layer(&self, layer: CanopyLayer) -> f32 {
        match layer {
            CanopyLayer::Understory => self.understory_moisture,
            CanopyLayer::Midstory => self.midstory_moisture,
            CanopyLayer::Overstory => self.overstory_moisture,
        }
    }

    /// Get total canopy height
    pub fn total_height(&self) -> f32 {
        if self.overstory_load > 0.0 {
            30.0 // Typical eucalyptus height
        } else if self.midstory_load > 0.0 {
            6.0
        } else {
            1.0
        }
    }
}

/// Calculate fire transition probability between layers
///
/// Based on:
/// - Fire intensity in lower layer
/// - Vertical fuel continuity (ladder fuels)
/// - Moisture in target layer
///
/// # Arguments
/// * `lower_layer_intensity` - Fire intensity in lower layer (kW/m)
/// * `canopy` - Canopy structure
/// * `from_layer` - Source layer
/// * `to_layer` - Target layer
///
/// # Returns
/// Transition probability (0-1)
pub fn calculate_layer_transition_probability(
    lower_layer_intensity: f32,
    canopy: &CanopyStructure,
    from_layer: CanopyLayer,
    to_layer: CanopyLayer,
) -> f32 {
    // Only support upward transitions
    let (from_min, _) = from_layer.height_range();
    let (to_min, _) = to_layer.height_range();

    if to_min <= from_min {
        return 0.0; // Can't transition downward or laterally
    }

    // Base transition threshold (kW/m)
    let base_threshold = match (from_layer, to_layer) {
        (CanopyLayer::Understory, CanopyLayer::Midstory) => 500.0,
        (CanopyLayer::Midstory, CanopyLayer::Overstory) => 2000.0,
        (CanopyLayer::Understory, CanopyLayer::Overstory) => 5000.0, // Direct jump (rare)
        _ => return 0.0,
    };

    // Adjust threshold based on ladder fuel continuity
    let adjusted_threshold = base_threshold * (1.0 - canopy.ladder_fuel_factor * 0.7);

    // Moisture penalty in target layer
    let target_moisture = canopy.moisture_at_layer(to_layer);
    let moisture_factor = (1.0 - target_moisture).max(0.0);

    // Calculate probability
    if lower_layer_intensity < adjusted_threshold * 0.5 {
        0.0 // Too weak
    } else if lower_layer_intensity > adjusted_threshold * 2.0 {
        moisture_factor // Strong enough to overcome moisture
    } else {
        // Gradual transition
        let intensity_factor =
            (lower_layer_intensity - adjusted_threshold * 0.5) / (adjusted_threshold * 1.5);
        intensity_factor * moisture_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_height_ranges() {
        assert!(CanopyLayer::Understory.contains_height(1.0));
        assert!(!CanopyLayer::Understory.contains_height(3.0));

        assert!(CanopyLayer::Midstory.contains_height(5.0));
        assert!(!CanopyLayer::Midstory.contains_height(1.0));

        assert!(CanopyLayer::Overstory.contains_height(15.0));
        assert!(!CanopyLayer::Overstory.contains_height(5.0));
    }

    #[test]
    fn test_stringybark_structure() {
        let canopy = CanopyStructure::eucalyptus_stringybark();

        // Should have fuel in all layers
        assert!(canopy.understory_load > 0.0);
        assert!(canopy.midstory_load > 0.0);
        assert!(canopy.overstory_load > 0.0);

        // Should have high ladder fuel factor
        assert!(canopy.ladder_fuel_factor > 0.7);

        // Moisture should increase with height
        assert!(canopy.understory_moisture < canopy.midstory_moisture);
        assert!(canopy.midstory_moisture < canopy.overstory_moisture);
    }

    #[test]
    fn test_smooth_bark_structure() {
        let canopy = CanopyStructure::eucalyptus_smooth_bark();

        // Should have low ladder fuel factor
        assert!(canopy.ladder_fuel_factor < 0.5);

        // Less midstory fuel than stringybark
        let stringybark = CanopyStructure::eucalyptus_stringybark();
        assert!(canopy.midstory_load < stringybark.midstory_load);
    }

    #[test]
    fn test_grassland_structure() {
        let canopy = CanopyStructure::grassland();

        // Only understory
        assert!(canopy.understory_load > 0.0);
        assert_eq!(canopy.midstory_load, 0.0);
        assert_eq!(canopy.overstory_load, 0.0);

        // No ladder fuels
        assert_eq!(canopy.ladder_fuel_factor, 0.0);
    }

    #[test]
    fn test_layer_transition_stringybark() {
        let canopy = CanopyStructure::eucalyptus_stringybark();

        // Strong fire should transition upward easily in stringybark
        let prob = calculate_layer_transition_probability(
            1000.0, // Strong fire
            &canopy,
            CanopyLayer::Understory,
            CanopyLayer::Midstory,
        );

        // Should have reasonable probability
        assert!(prob > 0.1, "Probability was {}", prob);
    }

    #[test]
    fn test_layer_transition_smooth_bark() {
        let canopy = CanopyStructure::eucalyptus_smooth_bark();

        // Same fire should transition less easily in smooth bark
        let prob = calculate_layer_transition_probability(
            1000.0,
            &canopy,
            CanopyLayer::Understory,
            CanopyLayer::Midstory,
        );

        let stringybark = CanopyStructure::eucalyptus_stringybark();
        let prob_stringybark = calculate_layer_transition_probability(
            1000.0,
            &stringybark,
            CanopyLayer::Understory,
            CanopyLayer::Midstory,
        );

        // Smooth bark should have lower transition probability
        assert!(prob < prob_stringybark);
    }

    #[test]
    fn test_weak_fire_no_transition() {
        let canopy = CanopyStructure::eucalyptus_stringybark();

        // Very weak fire shouldn't transition (even with high ladder fuel factor)
        let prob = calculate_layer_transition_probability(
            50.0, // Very weak fire
            &canopy,
            CanopyLayer::Understory,
            CanopyLayer::Midstory,
        );

        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_no_downward_transition() {
        let canopy = CanopyStructure::eucalyptus_stringybark();

        // Can't transition downward
        let prob = calculate_layer_transition_probability(
            5000.0,
            &canopy,
            CanopyLayer::Overstory,
            CanopyLayer::Midstory,
        );

        assert_eq!(prob, 0.0);
    }
}
